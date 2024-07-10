# tulsa

Learning Rust by building a scheduler for both async and sync tasks. This repo is separated into the following areas:
1. `tulsa`: scheduling library which has both thread and coroutine support
1. `app`: a sample REST API built on Axum which uses tulsa to run tasks
1. `load-test`: WIP experiments to compare the resource utilization of threads vs coroutines
