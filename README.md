# tulsa

Learning Rust by building a scheduler for both async and sync tasks. This repo is separated into the following areas:
1. `tulsa`: the scheduling library which has both thread and coroutine support
1. `example-app`: a sample REST API built using Axum and tulsa
1. `load-test`: WIP experiments to compare the resource utilization of threads vs coroutines
