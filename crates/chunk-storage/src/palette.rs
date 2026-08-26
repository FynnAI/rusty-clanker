use crate::bits::{ceil_log2, pack_bits, read_slot, unpack_bits, write_slot};
use crate::registry_id::{PaletteThresholds, RegistryId};

/// WORLD-D2's three palette states, illustrated identically in `03-world-chunks-
/// persistence.md` itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Palette<T> {
    SingleValue(T),
    Indirect { entries: Vec<T>, bits_per_entry: u8 },
    Direct { bits_per_entry: u16 },
}

/// The generic paletted container (WORLD-D2). One Rust type, two intended future
/// consumers (disk, WORLD-D2's own text; wire, `02-protocol-networking.md`) — though
/// only this crate's own in-memory/disk-facing consumer is wired up by this blueprint;
/// wire-encoder reuse is a future blueprint's integration (Context's Resolved
/// discrepancy — the wire encoder currently lives, hand-rolled and byte-compatible but
/// not type-shared, in `M1-B05`'s `crates/server/src/play/chunk.rs`).
#[derive(Clone, Debug)]
pub struct PalettedContainer<T: RegistryId> {
    palette: Palette<T>,
    data: Box<[u64]>,
    entry_count: u16,
    thresholds: PaletteThresholds,
}

impl<T: RegistryId> PalettedContainer<T> {
    /// A fresh container, every one of `entry_count` entries equal to `value`
    /// (`SingleValue`, zero data words — WORLD-D2). `entry_count` is `4096` for a
    /// block-state section, `64` for a biome section (Context).
    pub fn new_single(value: T, entry_count: u16, thresholds: PaletteThresholds) -> Self {
        Self {
            palette: Palette::SingleValue(value),
            data: Box::new([]),
            entry_count,
            thresholds,
        }
    }

    /// Reads the value at `index` (`0..entry_count`). Panics (via ordinary slice
    /// indexing) if `index >= entry_count`.
    pub fn get(&self, index: usize) -> T {
        match &self.palette {
            Palette::SingleValue(v) => *v,
            Palette::Indirect {
                entries,
                bits_per_entry,
            } => {
                let local = read_slot(&self.data, index, *bits_per_entry as u32);
                entries[local as usize]
            }
            Palette::Direct { bits_per_entry } => {
                T::from_raw(read_slot(&self.data, index, *bits_per_entry as u32))
            }
        }
    }

    /// Writes `value` at `index`, upgrading the palette strategy in place if needed
    /// (Implementation steps give the exact algorithm: `SingleValue -> Indirect`,
    /// `SingleValue -> Direct`, `Indirect` growth within itself, `Indirect -> Direct`,
    /// or a same-strategy in-place write). Returns `true` iff the value at `index`
    /// actually changed (Context's dirty-tracking hook).
    pub fn set(&mut self, index: usize, value: T) -> bool {
        let entry_count = self.entry_count as usize;
        let thresholds = self.thresholds;
        let Self { palette, data, .. } = self;

        match palette {
            Palette::SingleValue(v) => {
                if *v == value {
                    return false;
                }
                let old = *v;
                let bits = (thresholds.indirect_floor_bits as u32).max(ceil_log2(2));
                if bits <= thresholds.max_indirect_bits as u32 {
                    let mut locals = vec![0u32; entry_count];
                    locals[index] = 1;
                    *data = pack_bits(&locals, bits);
                    *palette = Palette::Indirect {
                        entries: vec![old, value],
                        bits_per_entry: bits as u8,
                    };
                } else {
                    let raws: Vec<u32> = (0..entry_count)
                        .map(|i| {
                            if i == index {
                                value.to_raw()
                            } else {
                                old.to_raw()
                            }
                        })
                        .collect();
                    *data = pack_bits(&raws, thresholds.direct_bits as u32);
                    *palette = Palette::Direct {
                        bits_per_entry: thresholds.direct_bits,
                    };
                }
                true
            }
            Palette::Indirect {
                entries,
                bits_per_entry,
            } => {
                if let Some(local) = entries.iter().position(|&e| e == value) {
                    let old_local = read_slot(data, index, *bits_per_entry as u32);
                    let changed = old_local != local as u32;
                    write_slot(data, index, local as u32, *bits_per_entry as u32);
                    changed
                } else {
                    let new_len = entries.len() + 1;
                    let new_bits =
                        (thresholds.indirect_floor_bits as u32).max(ceil_log2(new_len as u32));
                    if new_bits <= thresholds.max_indirect_bits as u32 {
                        if new_bits != *bits_per_entry as u32 {
                            let unpacked = unpack_bits(data, *bits_per_entry as u32, entry_count);
                            *data = pack_bits(&unpacked, new_bits);
                            *bits_per_entry = new_bits as u8;
                        }
                        entries.push(value);
                        let new_local = entries.len() - 1;
                        write_slot(data, index, new_local as u32, *bits_per_entry as u32);
                        true
                    } else {
                        let unpacked = unpack_bits(data, *bits_per_entry as u32, entry_count);
                        let mut raws: Vec<u32> = unpacked
                            .iter()
                            .map(|&local| entries[local as usize].to_raw())
                            .collect();
                        raws[index] = value.to_raw();
                        *data = pack_bits(&raws, thresholds.direct_bits as u32);
                        *palette = Palette::Direct {
                            bits_per_entry: thresholds.direct_bits,
                        };
                        true
                    }
                }
            }
            Palette::Direct { bits_per_entry } => {
                let old_raw = read_slot(data, index, *bits_per_entry as u32);
                let new_raw = value.to_raw();
                write_slot(data, index, new_raw, *bits_per_entry as u32);
                old_raw != new_raw
            }
        }
    }

    /// Read-only view of the current palette state — `Indirect`'s `entries`/
    /// `bits_per_entry`, `Direct`'s `bits_per_entry`, or the single `SingleValue`.
    pub fn palette(&self) -> &Palette<T> {
        &self.palette
    }

    /// The current bits-per-entry (`0` for `SingleValue`).
    pub fn bits_per_entry(&self) -> u16 {
        match &self.palette {
            Palette::SingleValue(_) => 0,
            Palette::Indirect { bits_per_entry, .. } => *bits_per_entry as u16,
            Palette::Direct { bits_per_entry } => *bits_per_entry,
        }
    }

    pub fn entry_count(&self) -> u16 {
        self.entry_count
    }

    /// The thresholds this container was constructed with (needed by a future
    /// serializer to know the registry's own `direct_bits` when re-deriving a palette
    /// from raw values — not otherwise used by this blueprint).
    pub fn thresholds(&self) -> PaletteThresholds {
        self.thresholds
    }

    /// Read-only access to the packed data words, exactly as `M1-B05`'s wire encoder
    /// would need to embed them (byte-compatibility — Context).
    pub fn raw_words(&self) -> &[u64] {
        &self.data
    }

    /// Iterates every entry's value, `index` ascending `0..entry_count`.
    pub fn iter(&self) -> Box<dyn Iterator<Item = T> + '_> {
        Box::new((0..self.entry_count as usize).map(|i| self.get(i)))
    }
}
