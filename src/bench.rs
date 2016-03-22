macro_rules! compare_box_and_arena {
    ($name: ident, $bytes: expr) => {
        mod $name {
            extern crate test;
            use ::Arena;

            const DUMMY_BYTE: u8 = 0x55;
            const ALLOCATION_COUNT: usize = 10_000;

            // Benchmarking is complicated by the fact that normal
            // Box allocation will immediately free the
            // allocation, making it so that the same memory page
            // will be used over and over.
            //
            // To avoid this, we place the allocated memory chunks
            // into a vector for all benchmarks, dropping them at
            // the end of the test.

            #[bench]
            fn with_nothing(b: &mut test::Bencher) {
                b.iter(|| {
                    for _ in 0..ALLOCATION_COUNT {
                        test::black_box([DUMMY_BYTE; $bytes]);
                    }
                })
            }

            #[bench]
            fn with_box(b: &mut test::Bencher) {
                let mut saved = Vec::with_capacity(ALLOCATION_COUNT);

                b.iter(|| {
                    for _ in 0..ALLOCATION_COUNT {
                        saved.push(test::black_box(Box::new([DUMMY_BYTE; $bytes])));
                    }
                })
            }

            #[bench]
            fn with_box_placement_syntax(b: &mut test::Bencher) {
                let mut saved = Vec::with_capacity(ALLOCATION_COUNT);

                b.iter(|| {
                    for _ in 0..ALLOCATION_COUNT {
                        saved.push(test::black_box(box [DUMMY_BYTE; $bytes]));
                    }
                })
            }

            #[bench]
            fn with_box_placement(b: &mut test::Bencher) {
                let mut saved = Vec::with_capacity(ALLOCATION_COUNT);

                b.iter(|| {
                    for _ in 0..ALLOCATION_COUNT {
                        saved.push(test::black_box(in ::std::boxed::HEAP { [DUMMY_BYTE; $bytes] }));
                    }
                })
            }

            #[bench]
            fn with_arena(b: &mut test::Bencher) {
                let arena = Arena::new();
                let mut saved = Vec::with_capacity(ALLOCATION_COUNT);

                b.iter(|| {
                    for _ in 0..ALLOCATION_COUNT {
                        saved.push(test::black_box(arena.alloc([DUMMY_BYTE; $bytes])));
                    }
                })
            }

            #[bench]
            fn with_arena_placement(b: &mut test::Bencher) {
                let arena = Arena::new();
                let mut saved = Vec::with_capacity(ALLOCATION_COUNT);

                b.iter(|| {
                    for _ in 0..ALLOCATION_COUNT {
                        saved.push(test::black_box(in & arena { [DUMMY_BYTE; $bytes] }));
                    }
                })
            }
        }
    };
}

compare_box_and_arena!(allocating_1_bytes, 1);
compare_box_and_arena!(allocating_2_bytes, 2);
compare_box_and_arena!(allocating_4_bytes, 4);
compare_box_and_arena!(allocating_8_bytes, 8);
compare_box_and_arena!(allocating_16_bytes, 16);
compare_box_and_arena!(allocating_32_bytes, 32);
compare_box_and_arena!(allocating_64_bytes, 64);
compare_box_and_arena!(allocating_128_bytes, 128);
compare_box_and_arena!(allocating_256_bytes, 256);
compare_box_and_arena!(allocating_512_bytes, 512);
compare_box_and_arena!(allocating_1024_bytes, 1024);
