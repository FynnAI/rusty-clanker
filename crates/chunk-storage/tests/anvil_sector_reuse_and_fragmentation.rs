//! Acceptance tests: sector-freeing on rewrite/shrink, `free_sector_summary`, and the
//! first-fit (never best-fit) allocation strategy — hand-verified sector arithmetic
//! throughout (M2-B03 Deliverables, `region_file.rs`).

mod support;

use rc_chunk_storage::RegionFile;
use support::TempWorldDir;

/// A payload of exactly `n` sectors' worth of on-disk record space: `4 (length) + 1
/// (tag) + data.len()` sums to exactly `n * 4096`, with no partial-sector rounding
/// ambiguity, so every hand-derived sector count in this file is exact.
fn payload_of_sectors(n: u32) -> Vec<u8> {
    vec![0xCDu8; (n as usize) * 4096 - 5]
}

fn file_sectors(path: &std::path::Path) -> u64 {
    let len = std::fs::metadata(path).unwrap().len();
    assert_eq!(len % 4096, 0);
    len / 4096
}

#[test]
fn shrinking_a_chunk_frees_its_excess_sectors_for_reuse() {
    let dir = TempWorldDir::new("shrinking_a_chunk_frees_its_excess_sectors_for_reuse");
    let path = dir.path().join("r.0.0.mca");
    let mut rf = RegionFile::open(path.clone(), 0, 0).unwrap();

    // A needs 3 sectors: file 2 -> 5 (offset 2).
    rf.write_record(0, 0, 3, &payload_of_sectors(3)).unwrap();
    assert_eq!(file_sectors(&path), 5);

    // B needs 1 sector: lands past A, file 5 -> 6 (offset 5).
    rf.write_record(1, 0, 3, &payload_of_sectors(1)).unwrap();
    assert_eq!(file_sectors(&path), 6);

    // Rewrite A with a payload needing only 1 sector: the always-fresh rule forbids
    // reusing A's own old 3-sector range in place, and no other free range exists yet
    // (A's old range still counts as claimed during this very scan), so this appends:
    // file 6 -> 7. A's old 3-sector range at offset 2 becomes free the moment this
    // write's header update lands.
    rf.write_record(0, 0, 3, &payload_of_sectors(1)).unwrap();
    assert_eq!(
        file_sectors(&path),
        7,
        "A's shrink-rewrite must append, not reuse in place"
    );

    // Write a brand-new chunk C needing 2 sectors: the 3-sector gap A's shrink left
    // behind (offset 2..5) satisfies it via first-fit reuse — the file must NOT grow
    // any further.
    rf.write_record(2, 0, 3, &payload_of_sectors(2)).unwrap();
    assert_eq!(
        file_sectors(&path),
        7,
        "C's allocation must be satisfied by reuse of A's freed gap, not by appending"
    );

    let (_, c_bytes) = rf.read_record(2, 0).unwrap().unwrap();
    assert_eq!(c_bytes, payload_of_sectors(2));
}

#[test]
fn free_sector_summary_reports_correct_range_count_and_total() {
    let dir = TempWorldDir::new("free_sector_summary_reports_correct_range_count_and_total");
    let path = dir.path().join("r.0.0.mca");
    let mut rf = RegionFile::open(path.clone(), 0, 0).unwrap();

    // Three sequential 1-sector writes at (0,0), (1,0), (2,0): each fully packs the
    // file with no gap, so each must append. Offsets 2, 3, 4; file grows to 5.
    rf.write_record(0, 0, 3, &payload_of_sectors(1)).unwrap();
    rf.write_record(1, 0, 3, &payload_of_sectors(1)).unwrap();
    rf.write_record(2, 0, 3, &payload_of_sectors(1)).unwrap();
    assert_eq!(file_sectors(&path), 5);
    assert_eq!(rf.free_sector_summary(), (0, 0));

    // Rewrite the middle one, (1,0), with a tiny (10-byte) payload — still 1 sector
    // needed, but the always-fresh rule moves it to a new offset (5), growing the
    // file to 6 and vacating its old offset-3 sector.
    rf.write_record(1, 0, 3, &[0u8; 10]).unwrap();
    assert_eq!(file_sectors(&path), 6);

    assert_eq!(rf.free_sector_summary(), (1, 1));
}

#[test]
fn fragmentation_first_fit_reuses_the_earlier_gap_even_when_a_later_gap_fits_more_tightly() {
    let dir = TempWorldDir::new(
        "fragmentation_first_fit_reuses_the_earlier_gap_even_when_a_later_gap_fits_more_tightly",
    );
    let path = dir.path().join("r.0.0.mca");
    let mut rf = RegionFile::open(path.clone(), 0, 0).unwrap();

    // (0,0) needs 5 sectors: offset 2, file -> 7.
    rf.write_record(0, 0, 3, &payload_of_sectors(5)).unwrap();
    assert_eq!(file_sectors(&path), 7);

    // (2,0) needs 1 sector: offset 7, file -> 8. This chunk's own allocation never
    // moves again for the rest of this test — it is the fixed spacer that keeps the
    // two gaps freed below from ever becoming adjacent (and therefore merging into
    // one, under `compute_free_ranges`' own maximal-contiguous-run scan, Context).
    rf.write_record(2, 0, 3, &payload_of_sectors(1)).unwrap();
    assert_eq!(file_sectors(&path), 8);

    // (1,0) needs 2 sectors: offset 8, file -> 10.
    rf.write_record(1, 0, 3, &payload_of_sectors(2)).unwrap();
    assert_eq!(file_sectors(&path), 10);

    // Rewrite (1,0) with a payload needing 3 sectors: no free range exists yet (the
    // file is fully packed 2..10), so this must extend: offset 10, file -> 13. This
    // frees the LATER, SMALLER gap at offset 8 (2 sectors) — bounded on its left by
    // (2,0)'s still-claimed sector 7, so it stays isolated.
    rf.write_record(1, 0, 3, &payload_of_sectors(3)).unwrap();
    assert_eq!(file_sectors(&path), 13);

    // Rewrite (0,0) with a payload needing 6 sectors: the only free range, (8, 2), is
    // too small, so this too must extend: offset 13, file -> 19. This frees the
    // EARLIER, LARGER gap at offset 2 (5 sectors) — bounded on its right by (2,0)'s
    // still-claimed sector 7, so it too stays isolated, never merging with (8, 2).
    rf.write_record(0, 0, 3, &payload_of_sectors(6)).unwrap();
    assert_eq!(file_sectors(&path), 19);

    // Exactly two free ranges now coexist: (offset 2, count 5) and (offset 8, count 2)
    // — separated by (2,0)'s claimed sector 7 in between, so they do not merge. Two
    // ranges, 5 + 2 = 7 free sectors total.
    assert_eq!(rf.free_sector_summary(), (2, 7));

    // Write a brand-new chunk (3,0) needing exactly 2 sectors: first-fit (scanning
    // from the lowest offset) must consume 2 sectors out of the EARLIER, LARGER range
    // at offset 2 — shrinking it to (4, 3) — never the later range at offset 8, which
    // exactly matches the request and a best-fit strategy would have chosen instead.
    rf.write_record(3, 0, 3, &payload_of_sectors(2)).unwrap();

    // No append: the file's total size is unchanged.
    assert_eq!(file_sectors(&path), 19);

    // Two ranges remain: the shrunk (4,3) plus the untouched (8,2) — 5 free sectors
    // total. A best-fit allocator would instead have fully consumed the offset-8
    // range, leaving one range, (2,5), and this same total of 5 — the range COUNT
    // alone distinguishes the two strategies unambiguously.
    assert_eq!(rf.free_sector_summary(), (2, 5));
}
