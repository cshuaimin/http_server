# http-server

A simple HTTP 1.0 server.

# Benchmark

## Hardware information:

CPU: i7-13700K @ 5.40 GHz, 24 Cores

Memory: 32GiB, 6400MHz

Disk: 1TiB SSD

## Command

```
cargo run --release -- 127.0.0.1 8000 . <threads>

wrk2 -t<threads> -c<threads> -d30s -R5000000 http://127.0.0.1:8000/Cargo.toml
```

## Results

Peak memory usage: 2MiB

| # threads | 1        | 2         | 4         | 8         | 16        | 24         | 32         |
|-----------|----------|-----------|-----------|-----------|-----------|------------|------------|
| Req/sec   | 86148.82 | 172808.29 | 323039.97 | 503680.75 | 734930.85 | 1079413.40 | 1078805.32 | 
