use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::anvil::error::StorageError;

/// The location table's per-slot entry packing: `(sector_offset: 24 bits) << 8 |
/// (sector_count: 8 bits)` (Context).
const HEADER_BYTES: usize = 8192;
const SECTOR_BYTES: usize = 4096;
const RECORD_SUBHEADER_BYTES: usize = 5; // 4-byte length + 1-byte compression tag
const EXTERNAL_BIT: u8 = 0x80;
const MAX_INLINE_SECTORS: u32 = 255;

/// One open `.mca` file: the 8 KiB header (decoded into two 1024-entry in-memory
/// tables) plus the underlying `std::fs::File`. NBT-agnostic by design (Context) — reads
/// and writes opaque `(compression_tag, bytes)` records; `AnvilDiskBackend` owns
/// compression selection and NBT validation. Not internally synchronized — callers
/// (`AnvilDiskBackend`) are responsible for the one-`parking_lot::Mutex`-per-handle
/// discipline (Context's Concurrency model).
pub struct RegionFile {
    file: File,
    /// The `.mca` file's own path — sibling `.mcc` overflow files are resolved
    /// relative to its parent directory (Context), and this is also the path every
    /// `StorageError::Io`/`Corrupt` variant this type produces names for diagnostics.
    path: PathBuf,
    region_x: i32,
    region_z: i32,
    locations: Box<[u32; 1024]>,
    timestamps: Box<[u32; 1024]>,
    file_sectors: u32,
}

impl RegionFile {
    /// Opens the `.mca` file at `path`, creating it (with an immediately-written, fresh
    /// all-zero 8 KiB header) if it does not already exist. `region_x`/`region_z` are
    /// the region's own grid coordinates (`chunk_x.div_euclid(32)` etc.) — supplied by
    /// the caller, not parsed from `path`'s filename, so this type never depends on any
    /// particular file-naming convention. Structural validity rule (Context): a
    /// pre-existing file of length `0` is treated as "not yet written" (same as
    /// freshly-created); length `1..8192` is `StorageError::Corrupt` ("shorter than the
    /// mandatory header"); length `>= 8192` not a multiple of `4096` is
    /// `StorageError::Corrupt` ("not sector-aligned"); anything else parses normally.
    pub fn open(path: PathBuf, region_x: i32, region_z: i32) -> Result<Self, StorageError> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|source| StorageError::Io {
                path: path.clone(),
                source,
            })?;

        let len = file
            .metadata()
            .map_err(|source| StorageError::Io {
                path: path.clone(),
                source,
            })?
            .len();

        if len == 0 {
            file.seek(SeekFrom::Start(0))
                .map_err(|source| StorageError::Io {
                    path: path.clone(),
                    source,
                })?;
            file.write_all(&[0u8; HEADER_BYTES])
                .map_err(|source| StorageError::Io {
                    path: path.clone(),
                    source,
                })?;
            file.sync_data().map_err(|source| StorageError::Io {
                path: path.clone(),
                source,
            })?;
            return Ok(Self {
                file,
                path,
                region_x,
                region_z,
                locations: Box::new([0u32; 1024]),
                timestamps: Box::new([0u32; 1024]),
                file_sectors: 2,
            });
        }

        if len < HEADER_BYTES as u64 {
            return Err(StorageError::Corrupt {
                path,
                reason: "shorter than the mandatory 8 KiB header".to_string(),
            });
        }
        if !len.is_multiple_of(SECTOR_BYTES as u64) {
            return Err(StorageError::Corrupt {
                path,
                reason: "file length is not sector-aligned (a multiple of 4096)".to_string(),
            });
        }

        let mut header = [0u8; HEADER_BYTES];
        file.seek(SeekFrom::Start(0))
            .map_err(|source| StorageError::Io {
                path: path.clone(),
                source,
            })?;
        file.read_exact(&mut header)
            .map_err(|source| StorageError::Io {
                path: path.clone(),
                source,
            })?;

        let mut locations = Box::new([0u32; 1024]);
        let mut timestamps = Box::new([0u32; 1024]);
        for i in 0..1024 {
            locations[i] = u32::from_be_bytes(header[i * 4..i * 4 + 4].try_into().unwrap());
            timestamps[i] =
                u32::from_be_bytes(header[4096 + i * 4..4096 + i * 4 + 4].try_into().unwrap());
        }

        Ok(Self {
            file,
            path,
            region_x,
            region_z,
            locations,
            timestamps,
            file_sectors: (len / SECTOR_BYTES as u64) as u32,
        })
    }

    /// Reads the record at local slot `(local_x, local_z)` (each `0..32`, already
    /// reduced modulo 32 by the caller). `Ok(None)` = empty slot (all-zero location
    /// entry) — never an error. `Ok(Some((tag, bytes)))` on success: `tag` is the raw
    /// on-disk compression-tag byte **including** the `0x80` external bit if it was set
    /// (the caller strips it before passing to `CompressionScheme::decompress_tagged`);
    /// `bytes` is the still-compressed payload (from the in-region sectors, or read
    /// whole from the paired `.mcc` file when external). Returns
    /// `StorageError::SectorOutOfBounds`/`Corrupt`/`MissingExternalFile` per Context's
    /// corruption-handling rules — a bad record at this one slot never affects any
    /// other slot's readability.
    pub fn read_record(
        &mut self,
        local_x: u8,
        local_z: u8,
    ) -> Result<Option<(u8, Vec<u8>)>, StorageError> {
        let idx = Self::slot_index(local_x, local_z);
        let entry = self.locations[idx];
        if entry == 0 {
            return Ok(None);
        }

        let offset = entry >> 8;
        let count = (entry & 0xFF) as u8;

        if offset < 2 || offset as u64 + count as u64 > self.file_sectors as u64 {
            return Err(StorageError::SectorOutOfBounds {
                local_x,
                local_z,
                offset,
                count,
                file_sectors: self.file_sectors,
            });
        }

        let mut block = vec![0u8; count as usize * SECTOR_BYTES];
        self.file
            .seek(SeekFrom::Start(offset as u64 * SECTOR_BYTES as u64))
            .map_err(|source| self.io_err(source))?;
        self.file
            .read_exact(&mut block)
            .map_err(|source| self.io_err(source))?;

        if block.len() < RECORD_SUBHEADER_BYTES {
            return Err(StorageError::Corrupt {
                path: self.path.clone(),
                reason: "record block shorter than the 5-byte length/tag sub-header".to_string(),
            });
        }

        let length = u32::from_be_bytes(block[0..4].try_into().unwrap());
        if length == 0 {
            return Err(StorageError::Corrupt {
                path: self.path.clone(),
                reason: "record's own length field is zero".to_string(),
            });
        }
        let raw_tag = block[4];

        if raw_tag & EXTERNAL_BIT != 0 {
            let mcc_path = self.mcc_path(local_x, local_z);
            let bytes = std::fs::read(&mcc_path)
                .map_err(|_| StorageError::MissingExternalFile { path: mcc_path })?;
            Ok(Some((raw_tag, bytes)))
        } else {
            let payload_len = length as usize - 1;
            if RECORD_SUBHEADER_BYTES + payload_len > block.len() {
                return Err(StorageError::Corrupt {
                    path: self.path.clone(),
                    reason: format!(
                        "declared length {length} exceeds the {} sector(s) actually allocated",
                        count
                    ),
                });
            }
            let payload =
                block[RECORD_SUBHEADER_BYTES..RECORD_SUBHEADER_BYTES + payload_len].to_vec();
            Ok(Some((raw_tag, payload)))
        }
    }

    /// Writes `data` (already compressed by the caller) under `compression_tag`'s low 7
    /// bits (**without** the `0x80` bit — this method decides internally, from `data`'s
    /// own length against the 255-sector cap, whether the record must go external, sets
    /// the bit itself, and writes `data` verbatim to the paired `.mcc` file when it
    /// does). Implements the crash-safe always-fresh-allocation algorithm exactly
    /// (Context). Cleans up (best-effort) a stale `.mcc` file when this write is
    /// non-external (Context, step 7).
    pub fn write_record(
        &mut self,
        local_x: u8,
        local_z: u8,
        compression_tag: u8,
        data: &[u8],
    ) -> Result<(), StorageError> {
        let idx = Self::slot_index(local_x, local_z);
        let mcc_path = self.mcc_path(local_x, local_z);

        let total_inline = RECORD_SUBHEADER_BYTES + data.len();
        let goes_external = total_inline > MAX_INLINE_SECTORS as usize * SECTOR_BYTES;

        let (sector_buffer, sectors_needed) = if goes_external {
            let mut mcc_file = File::create(&mcc_path).map_err(|source| StorageError::Io {
                path: mcc_path.clone(),
                source,
            })?;
            mcc_file
                .write_all(data)
                .map_err(|source| StorageError::Io {
                    path: mcc_path.clone(),
                    source,
                })?;
            mcc_file.sync_data().map_err(|source| StorageError::Io {
                path: mcc_path.clone(),
                source,
            })?;
            drop(mcc_file);

            (
                Self::build_sector_buffer(&[], compression_tag | EXTERNAL_BIT, 1),
                1u32,
            )
        } else {
            // Best-effort cleanup of a stale `.mcc` file left by a previous, larger
            // write of this same slot (Context, step 7) — a `NotFound` (or any other)
            // error here is a no-op by design.
            let _ = std::fs::remove_file(&mcc_path);

            let sectors = (total_inline as u32).div_ceil(SECTOR_BYTES as u32);
            (
                Self::build_sector_buffer(data, compression_tag, sectors),
                sectors,
            )
        };

        // Step 2/3: scan the file's *current, unmodified* location table for a free
        // range, first-fit; append at end-of-file if none fits (Context).
        let offset = self.allocate_sectors(sectors_needed);
        let write_end = offset + sectors_needed;
        if write_end > self.file_sectors {
            self.file_sectors = write_end;
        }

        // Step 4/5: write the sector-aligned record buffer, then `sync_data`.
        self.file
            .seek(SeekFrom::Start(offset as u64 * SECTOR_BYTES as u64))
            .map_err(|source| self.io_err(source))?;
        self.file
            .write_all(&sector_buffer)
            .map_err(|source| self.io_err(source))?;
        self.file
            .sync_data()
            .map_err(|source| self.io_err(source))?;

        // Step 6: update in-memory + on-disk location/timestamp entries, `sync_data`
        // again. Only now does the old range stop being protected by the header.
        let packed_entry = (offset << 8) | (sectors_needed & 0xFF);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as u32;

        self.locations[idx] = packed_entry;
        self.timestamps[idx] = now;

        self.file
            .seek(SeekFrom::Start((idx * 4) as u64))
            .map_err(|source| self.io_err(source))?;
        self.file
            .write_all(&packed_entry.to_be_bytes())
            .map_err(|source| self.io_err(source))?;
        self.file
            .seek(SeekFrom::Start((4096 + idx * 4) as u64))
            .map_err(|source| self.io_err(source))?;
        self.file
            .write_all(&now.to_be_bytes())
            .map_err(|source| self.io_err(source))?;
        self.file
            .sync_data()
            .map_err(|source| self.io_err(source))?;

        Ok(())
    }

    /// This slot's last-write Unix timestamp (seconds), or `None` if never written.
    pub fn timestamp(&self, local_x: u8, local_z: u8) -> Option<u32> {
        let idx = Self::slot_index(local_x, local_z);
        if self.locations[idx] == 0 {
            None
        } else {
            Some(self.timestamps[idx])
        }
    }

    /// `(free_range_count, total_free_sectors)` — exposed for this blueprint's own
    /// sector-reuse/fragmentation acceptance tests; recomputed fresh on every call
    /// (Context: no persisted free-list exists to introspect).
    pub fn free_sector_summary(&self) -> (usize, u32) {
        let ranges = self.compute_free_ranges();
        let total = ranges.iter().map(|&(_, count)| count).sum();
        (ranges.len(), total)
    }

    /// The location-table byte offset for a given local slot, per Context's own
    /// `local_x + 32 * local_z` formula.
    fn slot_index(local_x: u8, local_z: u8) -> usize {
        local_z as usize * 32 + local_x as usize
    }

    /// The sibling `.mcc` overflow file path for `(local_x, local_z)`: absolute chunk
    /// coordinates, in the same directory as this `.mca` file (Context).
    fn mcc_path(&self, local_x: u8, local_z: u8) -> PathBuf {
        let abs_x = self.region_x * 32 + local_x as i32;
        let abs_z = self.region_z * 32 + local_z as i32;
        let dir = self.path.parent().unwrap_or_else(|| Path::new("."));
        dir.join(format!("c.{abs_x}.{abs_z}.mcc"))
    }

    fn io_err(&self, source: std::io::Error) -> StorageError {
        StorageError::Io {
            path: self.path.clone(),
            source,
        }
    }

    /// Builds a sector-aligned, zero-padded record buffer: `[length: u32 BE]
    /// [compression_tag: u8][data]` (Context).
    fn build_sector_buffer(data: &[u8], tag: u8, sectors: u32) -> Vec<u8> {
        let length = 1u32 + data.len() as u32;
        let mut buf = vec![0u8; sectors as usize * SECTOR_BYTES];
        buf[0..4].copy_from_slice(&length.to_be_bytes());
        buf[4] = tag;
        buf[RECORD_SUBHEADER_BYTES..RECORD_SUBHEADER_BYTES + data.len()].copy_from_slice(data);
        buf
    }

    /// This write's target sector offset: the first-fit free range at least
    /// `sectors_needed` long, or `self.file_sectors` (append) if none fits (Context's
    /// exact algorithm — this scan always runs against the *current*, not-yet-mutated
    /// location table, so the chunk's own old range, if any, still counts as claimed).
    fn allocate_sectors(&self, sectors_needed: u32) -> u32 {
        for (offset, count) in self.compute_free_ranges() {
            if count >= sectors_needed {
                return offset;
            }
        }
        self.file_sectors
    }

    /// Recomputes every free sector range in `[2, file_sectors)` fresh from the current
    /// 1024 location entries (Context: no persisted free-list ever exists). Claims are
    /// clipped to `[2, file_sectors)` so a corrupt or out-of-range entry elsewhere can
    /// never cause an out-of-bounds index or a panic here.
    fn compute_free_ranges(&self) -> Vec<(u32, u32)> {
        let file_sectors = self.file_sectors;
        if file_sectors <= 2 {
            return Vec::new();
        }

        let mut claimed = vec![false; file_sectors as usize];
        for &entry in self.locations.iter() {
            if entry == 0 {
                continue;
            }
            let offset = entry >> 8;
            let count = entry & 0xFF;
            let start = offset.clamp(2, file_sectors);
            let end = offset.saturating_add(count).min(file_sectors);
            for sector in start..end {
                claimed[sector as usize] = true;
            }
        }

        let mut ranges = Vec::new();
        let mut i = 2usize;
        while i < file_sectors as usize {
            if claimed[i] {
                i += 1;
                continue;
            }
            let start = i;
            while i < file_sectors as usize && !claimed[i] {
                i += 1;
            }
            ranges.push((start as u32, (i - start) as u32));
        }
        ranges
    }
}
