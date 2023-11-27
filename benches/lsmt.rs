use criterion::{criterion_group, criterion_main, Criterion};
use lsm_tree::Config;
use rand::Rng;
use tempfile::tempdir;

/* fn insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("inserts");
    group.sample_size(10);

    let item_count = 100_000;

    for thread_count in [1_u32, 2, 4, 8] {
        group.bench_function(
            format!("{} inserts ({} threads)", item_count, thread_count),
            |b| {
                let tree = Config::new(tempdir().unwrap()).open().unwrap();

                b.iter(|| {
                    let mut threads = vec![];

                    for _ in 0..thread_count {
                        let tree = tree.clone();

                        threads.push(std::thread::spawn(move || {
                            for _ in 0..item_count {
                                let key = nanoid::nanoid!();
                                let value = nanoid::nanoid!();
                                tree.insert(key, value).unwrap();
                            }
                        }));
                    }

                    for thread in threads {
                        thread.join().unwrap();
                    }
                })
            },
        );
    }
} */

fn memtable_point_reads(c: &mut Criterion) {
    let mut group = c.benchmark_group("memtable point reads");

    let tree = Config::new(tempdir().unwrap()).open().unwrap();

    let max = 1_000_000;
    let lookup_count = 100_000;

    for x in 0_u32..max {
        let key = x.to_be_bytes();
        let value = nanoid::nanoid!();
        tree.insert(key, value).unwrap();
    }

    assert_eq!(tree.len().unwrap() as u32, max);

    for thread_count in [1_u32, 2, 4, 8] {
        group.bench_function(
            format!("{} point reads ({} threads)", lookup_count, thread_count),
            |b| {
                b.iter(|| {
                    let mut threads = vec![];

                    for _ in 0..thread_count {
                        let tree = tree.clone();

                        threads.push(std::thread::spawn(move || {
                            let mut rng = rand::thread_rng();

                            for _ in 0_u32..(lookup_count / thread_count) {
                                let key = rng.gen_range(0..max);
                                assert!(tree.get(key.to_be_bytes()).unwrap().is_some());
                            }
                        }));
                    }

                    for thread in threads {
                        thread.join().unwrap();
                    }
                })
            },
        );
    }
}

fn disk_point_reads(c: &mut Criterion) {
    let mut group = c.benchmark_group("disk point reads");
    group.sample_size(10);

    let tree = Config::new(tempdir().unwrap())
        .block_cache_size(0)
        .open()
        .unwrap();

    let max = 1_000_000;
    let lookup_count = 100_000;

    for x in 0_u32..max {
        let key = x.to_be_bytes();
        let value = nanoid::nanoid!();
        tree.insert(key, value).unwrap();
    }

    tree.force_memtable_flush()
        .expect("Flush error")
        .join()
        .expect("Join failed")
        .expect("Flush thread failed");

    assert_eq!(tree.len().unwrap() as u32, max);

    for thread_count in [1_u32, 2, 4, 8] {
        group.bench_function(
            format!("{} point reads ({} threads)", lookup_count, thread_count),
            |b| {
                b.iter(|| {
                    let mut threads = vec![];

                    for _ in 0..thread_count {
                        let tree = tree.clone();

                        threads.push(std::thread::spawn(move || {
                            let mut rng = rand::thread_rng();

                            for _ in 0_u32..(lookup_count / thread_count) {
                                let key = rng.gen_range(0..max);
                                assert!(tree.get(key.to_be_bytes()).unwrap().is_some());
                            }
                        }));
                    }

                    for thread in threads {
                        thread.join().unwrap();
                    }
                })
            },
        );
    }
}

fn cached_retrieve_disk_random(c: &mut Criterion) {
    let mut group = c.benchmark_group("disk point reads (with cache)");
    group.sample_size(10);

    let tree = Config::new(tempdir().unwrap())
        .block_cache_size(62 * 1_000) // 256 MB
        .open()
        .unwrap();

    let max = 1_000_000;
    let lookup_count = 100_000;

    for x in 0_u32..max {
        let key = x.to_be_bytes();
        let value = nanoid::nanoid!();
        tree.insert(key, value).unwrap();
    }

    tree.force_memtable_flush()
        .expect("Flush error")
        .join()
        .expect("Join failed")
        .expect("Flush thread failed");
    assert_eq!(tree.len().unwrap() as u32, max);

    for thread_count in [1_u32, 2, 4, 8] {
        group.bench_function(
            format!("{} point reads ({} threads)", lookup_count, thread_count),
            |b| {
                b.iter(|| {
                    let mut threads = vec![];

                    for _ in 0..thread_count {
                        let tree = tree.clone();

                        threads.push(std::thread::spawn(move || {
                            let mut rng = rand::thread_rng();

                            for _ in 0_u32..(lookup_count / thread_count) {
                                let key = rng.gen_range(0..max);
                                assert!(tree.get(key.to_be_bytes()).unwrap().is_some());
                            }
                        }));
                    }

                    for thread in threads {
                        thread.join().unwrap();
                    }
                })
            },
        );
    }
}

fn full_scan(c: &mut Criterion) {
    let mut group = c.benchmark_group("full scan");
    group.sample_size(10);

    let item_count = 500_000;

    group.bench_function("full scan uncached", |b| {
        let tree = Config::new(tempdir().unwrap()).open().unwrap();

        for x in 0_u32..item_count {
            let key = x.to_be_bytes();
            let value = nanoid::nanoid!();
            tree.insert(key, value).expect("Insert error");
        }

        tree.force_memtable_flush()
            .expect("Flush error")
            .join()
            .expect("Join failed")
            .expect("Flush thread failed");

        b.iter(|| {
            assert_eq!(tree.len().unwrap(), item_count as usize);
        })
    });

    group.bench_function("full scan cached", |b| {
        let tree = Config::new(tempdir().unwrap())
            .block_cache_size(100_000)
            .open()
            .unwrap();

        for x in 0_u32..item_count {
            let key = x.to_be_bytes();
            let value = nanoid::nanoid!();
            tree.insert(key, value).expect("Insert error");
        }

        tree.force_memtable_flush()
            .expect("Flush error")
            .join()
            .expect("Join failed")
            .expect("Flush thread failed");
        assert_eq!(tree.len().unwrap(), item_count as usize);

        b.iter(|| {
            assert_eq!(tree.len().unwrap(), item_count as usize);
        })
    });
}

fn scan_vs_query(c: &mut Criterion) {
    use std::ops::Bound::*;

    let mut group = c.benchmark_group("scan vs query");

    for size in [100_000, 1_000_000, 2_000_000, 5_000_000] {
        let db = Config::new(tempdir().unwrap()).open().unwrap();

        for x in 0..size as u64 {
            let key = x.to_be_bytes().to_vec();
            let value = nanoid::nanoid!().as_bytes().to_vec();
            db.insert(key, value).expect("Insert error");
        }

        db.force_memtable_flush()
            .expect("Flush error")
            .join()
            .expect("Join failed")
            .expect("Flush thread failed");
        assert_eq!(db.len().unwrap(), size);

        group.sample_size(10);
        group.bench_function(format!("scan {}", size), |b| {
            b.iter(|| {
                let iter = db.iter().unwrap();
                let iter = iter.into_iter();
                let count = iter
                    .filter(|x| match x {
                        Ok(item) => {
                            let buf = &item.key[..8];
                            let (int_bytes, _rest) = buf.split_at(std::mem::size_of::<u64>());
                            let num = u64::from_be_bytes(int_bytes.try_into().unwrap());
                            (60000..61000).contains(&num)
                        }
                        Err(_) => false,
                    })
                    .count();
                assert_eq!(count, 1000);
            })
        });
        group.bench_function(format!("query {}", size), |b| {
            b.iter(|| {
                let iter = db
                    .range((
                        Included(60000_u64.to_be_bytes().to_vec()),
                        Excluded(61000_u64.to_be_bytes().to_vec()),
                    ))
                    .unwrap();
                let iter = iter.into_iter();
                assert_eq!(iter.count(), 1000);
            })
        });
        group.bench_function(format!("query rev {}", size), |b| {
            b.iter(|| {
                let iter = db
                    .range((
                        Included(60000_u64.to_be_bytes().to_vec()),
                        Excluded(61000_u64.to_be_bytes().to_vec()),
                    ))
                    .unwrap();
                let iter = iter.into_iter();
                assert_eq!(iter.rev().count(), 1000);
            })
        });
    }
}

criterion_group!(
    benches,
    // insert,
    memtable_point_reads,
    disk_point_reads,
    cached_retrieve_disk_random,
    full_scan,
    scan_vs_query
);

criterion_main!(benches);