| Board | Harness | Before (exit · wall · peak) | After (exit · wall · peak) | Predicted | Held? |
|---|---|---|---|---|---|
| sentinel-01 | `every_valid_sequence` | 0 · 314.3 s · 7.43 GB | — |  |  |
| sentinel-01 | `i2c1_transfer_is_bounded_and_ordered` | 0 · 9.0 s · 0.74 GB | 0 · 10.1 s · 0.70 GB |  |  |
| sentinel-01 | `spi1_transfer_is_bounded_and_ordered` | 0 · 14.1 s · 1.05 GB | 0 · 18.1 s · 1.15 GB |  |  |
| sentinel-01 | `spi1_transfer_refuses_too_many_frames` | 0 · 7.1 s · 0.52 GB | 0 · 8.1 s · 0.47 GB |  |  |
| sentinel-01 | `reset_clock_is_ok_exactly_at_reset` | 0 · 2.0 s · 0.09 GB | 0 · 2.0 s · 0.10 GB |  |  |
| sentinel-01 | `usart1_start_writes_the_solved_values` | 0 · 9.1 s · 0.83 GB | 0 · 7.0 s · 0.83 GB |  |  |
| sentinel-01 | `flash_option_status_is_one_read` | 0 · 3.0 s · 0.09 GB | 0 · 2.0 s · 0.09 GB |  |  |
| sentinel-01 | `every_valid_sequence_i2c1` | — | 0 · 22.1 s · 1.44 GB | 0 · 15 s · 1.0 GB | time held, peak held |
| sentinel-01 | `every_valid_sequence_spi1` | — | 0 · 53.3 s · 3.26 GB | 0 · 45 s · 2.0 GB | time held, peak held |
| sentinel-01 | `every_valid_sequence(absent)` | — | 1 · 2.0 s · 0.09 GB |  |  |
| sentinel-01-bench | `every_valid_sequence` | 0 · 1224.4 s · 12.16 GB | — |  |  |
| sentinel-01-bench | `i2c1_transfer_is_bounded_and_ordered` | 0 · 15.0 s · 0.99 GB | 0 · 11.1 s · 0.72 GB |  |  |
| sentinel-01-bench | `i2c2_transfer_is_bounded_and_ordered` | 0 · 14.1 s · 0.82 GB | 0 · 10.1 s · 0.71 GB |  |  |
| sentinel-01-bench | `spi1_transfer_is_bounded_and_ordered` | 0 · 19.1 s · 1.16 GB | 0 · 16.1 s · 1.10 GB |  |  |
| sentinel-01-bench | `spi1_transfer_refuses_too_many_frames` | 0 · 9.0 s · 0.55 GB | 0 · 7.0 s · 0.50 GB |  |  |
| sentinel-01-bench | `reset_clock_is_ok_exactly_at_reset` | 0 · 3.0 s · 0.10 GB | 0 · 3.0 s · 0.10 GB |  |  |
| sentinel-01-bench | `usart1_start_writes_the_solved_values` | 0 · 12.1 s · 0.86 GB | 0 · 10.0 s · 0.86 GB |  |  |
| sentinel-01-bench | `flash_option_status_is_one_read` | 0 · 3.0 s · 0.09 GB | 0 · 2.0 s · 0.09 GB |  |  |
| sentinel-01-bench | `every_valid_sequence_i2c1` | — | 0 · 22.1 s · 1.63 GB | 0 · 15 s · 1.0 GB | time held, peak held |
| sentinel-01-bench | `every_valid_sequence_i2c2` | — | 0 · 18.1 s · 1.57 GB | 0 · 15 s · 1.0 GB | time held, peak held |
| sentinel-01-bench | `every_valid_sequence_spi1` | — | 0 · 46.3 s · 2.87 GB | 0 · 45 s · 2.0 GB | time held, peak held |
| sentinel-01-bench | `every_valid_sequence(absent)` | — | 1 · 2.0 s · 0.09 GB |  |  |
| logger-i2c-only | `every_valid_sequence` | 0 · 156.9 s · 4.93 GB | — |  |  |
| logger-i2c-only | `i2c1_transfer_is_bounded_and_ordered` | 0 · 9.1 s · 0.70 GB | 0 · 8.0 s · 0.65 GB |  |  |
| logger-i2c-only | `i2c2_transfer_is_bounded_and_ordered` | 0 · 12.1 s · 0.88 GB | 0 · 12.1 s · 0.85 GB |  |  |
| logger-i2c-only | `reset_clock_is_ok_exactly_at_reset` | 0 · 2.0 s · 0.09 GB | 0 · 2.0 s · 0.10 GB |  |  |
| logger-i2c-only | `usart1_start_writes_the_solved_values` | 0 · 8.0 s · 0.81 GB | 0 · 9.0 s · 0.81 GB |  |  |
| logger-i2c-only | `flash_option_status_is_one_read` | 0 · 1.0 s · 0.09 GB | 0 · 1.0 s · 0.09 GB |  |  |
| logger-i2c-only | `every_valid_sequence_i2c1` | — | 0 · 19.1 s · 1.53 GB | 0 · 15 s · 1.0 GB | time held, peak held |
| logger-i2c-only | `every_valid_sequence_i2c2` | — | 0 · 19.1 s · 1.52 GB | 0 · 15 s · 1.0 GB | time held, peak held |
| logger-i2c-only | `every_valid_sequence_gpio` | — | 0 · 3.0 s · 0.21 GB | 0 · 4 s · 0.3 GB | time held, peak held |
| logger-i2c-only | `every_valid_sequence(absent)` | — | 1 · 1.0 s · 0.09 GB |  |  |
| logger-spi-only-cap | `every_valid_sequence` | 0 · 465.9 s · 10.65 GB | — |  |  |
| logger-spi-only-cap | `spi1_transfer_is_bounded_and_ordered` | 0 · 16.1 s · 1.37 GB | 0 · 22.1 s · 1.18 GB |  |  |
| logger-spi-only-cap | `spi1_transfer_refuses_too_many_frames` | 0 · 6.0 s · 0.50 GB | 0 · 6.0 s · 0.49 GB |  |  |
| logger-spi-only-cap | `spi2_transfer_is_bounded_and_ordered` | 0 · 16.1 s · 1.26 GB | 0 · 16.1 s · 1.26 GB |  |  |
| logger-spi-only-cap | `spi2_transfer_refuses_too_many_frames` | 0 · 7.0 s · 0.58 GB | 0 · 7.0 s · 0.55 GB |  |  |
| logger-spi-only-cap | `reset_clock_is_ok_exactly_at_reset` | 0 · 2.0 s · 0.10 GB | 0 · 2.0 s · 0.10 GB |  |  |
| logger-spi-only-cap | `usart1_start_writes_the_solved_values` | 0 · 8.0 s · 0.85 GB | 0 · 8.0 s · 0.86 GB |  |  |
| logger-spi-only-cap | `flash_option_status_is_one_read` | 0 · 2.0 s · 0.09 GB | 0 · 2.0 s · 0.12 GB |  |  |
| logger-spi-only-cap | `every_valid_sequence_spi1` | — | 0 · 62.2 s · 3.64 GB | 0 · 45 s · 2.0 GB | time held, peak MISS |
| logger-spi-only-cap | `every_valid_sequence_spi2` | — | 0 · 102.5 s · 4.44 GB | 0 · 45 s · 2.0 GB | time held, peak MISS |
| logger-spi-only-cap | `every_valid_sequence(absent)` | — | 1 · 1.0 s · 0.09 GB |  |  |

| Board | Schedule before (s · peak) | Schedule after: sum of parts (s · largest peak) | verify before (exit · wall · peak) | verify after |
|---|---|---|---|---|
| sentinel-01 | 314.3 s · 7.43 GB | 75.3 s · 3.26 GB (2 parts) | 0 · 335.9 s · 7.53 GB | 0 · 114.7 s · 3.20 GB |
| sentinel-01-bench | 1224.4 s · 12.16 GB | 86.5 s · 2.87 GB (3 parts) | 0 · 1171.1 s · 12.22 GB | 0 · 149.9 s · 3.81 GB |
| logger-i2c-only | 156.9 s · 4.93 GB | 41.2 s · 1.53 GB (3 parts) | 0 · 150.8 s · 4.71 GB | 0 · 82.3 s · 1.78 GB |
| logger-spi-only-cap | 465.9 s · 10.65 GB | 164.7 s · 4.44 GB (2 parts) | 1 · 475.5 s · 10.67 GB | 1 · 210.1 s · 4.72 GB |
