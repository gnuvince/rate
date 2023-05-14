rate
====

A simple CLI tool to display how much a data rate is with different periods.

Usage
-----

```
$ rate 10 MB / s
 10.000 MB / sec
600.000 MB / min
 36.000 GB / hour
864.000 GB / day
  6.048 TB / week
 25.920 TB / month
315.360 TB / year

$ rate 14tb/day
162.037 MB / sec
  9.722 GB / min
583.333 GB / hour
 14.000 TB / day
 98.000 TB / week
420.000 TB / month
  5.110 PB / year

$ rate -h
Usage: rate <number> <unit> / <period>
       <number>: integer or float (no scientific notation)
       <unit>  : B KB MB GB TB PB EB ZB YB
       <period>: sec min hour day week month year
```

Installation
------------

To install `rate`, clone this repository and run `cargo install --path /path/to/rate/repo`.

License
-------

rate is distributed under the terms of the MIT license.
See LICENSE for the details.
