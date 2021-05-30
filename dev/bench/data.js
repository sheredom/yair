window.BENCHMARK_DATA = {
  "lastUpdate": 1622392972601,
  "repoUrl": "https://github.com/sheredom/yair",
  "entries": {
    "Rust Benchmark": [
      {
        "commit": {
          "author": {
            "name": "sheredom",
            "username": "sheredom"
          },
          "committer": {
            "name": "sheredom",
            "username": "sheredom"
          },
          "id": "77e0e7b01f4bb68d65460234821bae1ef50762c2",
          "message": "try run a benchmark on CI",
          "timestamp": "2021-05-26T14:24:35Z",
          "url": "https://github.com/sheredom/yair/pull/13/commits/77e0e7b01f4bb68d65460234821bae1ef50762c2"
        },
        "date": 1622043194268,
        "tool": "cargo",
        "benches": [
          {
            "name": "llvm__benchmarks__tests__bench_splat_adds",
            "value": 3463975,
            "range": "± 710923",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "611171+sheredom@users.noreply.github.com",
            "name": "Neil Henning",
            "username": "sheredom"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "e3c87ba193ffb4c17d5bcee15a67826ab2eedb76",
          "message": "Merge pull request #13 from sheredom/benchmark\n\ntry run a benchmark on CI",
          "timestamp": "2021-05-26T16:41:07+01:00",
          "tree_id": "8f7812d1710e762389cde110824b9961a2c98e20",
          "url": "https://github.com/sheredom/yair/commit/e3c87ba193ffb4c17d5bcee15a67826ab2eedb76"
        },
        "date": 1622043981415,
        "tool": "cargo",
        "benches": [
          {
            "name": "llvm__benchmarks__tests__bench_splat_adds",
            "value": 2866901,
            "range": "± 651156",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "neil.henning@hey.com",
            "name": "Neil Henning",
            "username": "sheredom"
          },
          "committer": {
            "email": "neil.henning@hey.com",
            "name": "Neil Henning",
            "username": "sheredom"
          },
          "distinct": true,
          "id": "a3cd2287fdc0e66605c72b7c9b716ad41084b765",
          "message": "More benchmarks.",
          "timestamp": "2021-05-30T17:32:21+01:00",
          "tree_id": "e14a2469f887fe579e0a8377ef011a0e6984c2c6",
          "url": "https://github.com/sheredom/yair/commit/a3cd2287fdc0e66605c72b7c9b716ad41084b765"
        },
        "date": 1622392971416,
        "tool": "cargo",
        "benches": [
          {
            "name": "io__benchmarks__tests__create_block",
            "value": 51,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "io__benchmarks__tests__create_function",
            "value": 174,
            "range": "± 82",
            "unit": "ns/iter"
          },
          {
            "name": "io__benchmarks__tests__create_global",
            "value": 158,
            "range": "± 35",
            "unit": "ns/iter"
          },
          {
            "name": "io__benchmarks__tests__create_instruction",
            "value": 64,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "io__benchmarks__tests__create_library",
            "value": 549,
            "range": "± 211",
            "unit": "ns/iter"
          },
          {
            "name": "io__benchmarks__tests__create_module",
            "value": 43,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "llvm__benchmarks__tests__bench_splat_adds",
            "value": 3252307,
            "range": "± 361475",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}