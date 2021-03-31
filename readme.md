# rust-ipv4-dns-resolver

A rust application to quickly resolve all the IPv4 addresses from multiple files of hostnames.

## Description

Is this one of my first multi threaded applications (originally written in C using pthreads for my Operating Systems class). This is using a combination of mutexes and condition variables to solve the classic bounded buffer (producer/consumer) problem, without deadlock and starvation.

## Usage

```bash
ipv4-resolver 10 10 serviced.txt resolved.txt input/*.txt
```
