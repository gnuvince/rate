use std::env;
use std::process::exit;

const PROG_NAME: &str = env!("CARGO_BIN_NAME");
const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
const SECOND: u64 = 1;
const MINUTE: u64 = 60 * SECOND;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;
const WEEK: u64 = 7 * DAY;
const MONTH: u64 = 30 * DAY;
const YEAR: u64 = 365 * DAY;
const PERIODS: &[u64] = &[SECOND, MINUTE, HOUR, DAY, WEEK, MONTH, YEAR];
const PERIOD_NAMES: &[&str] = &["sec", "min", "hour", "day", "week", "month", "year"];

fn main() {
    let mut s = String::new();
    for a in env::args().skip(1) {
        s.push_str(a.as_str());
    }
    match parse(&s) {
        Ok(res) => {
            let normalized = res.normalized();
            for i in 0..PERIODS.len() {
                let period = PERIODS[i];
                let period_name = PERIOD_NAMES[i];
                let (rate, unit) = nearest_power_of_1000_rate(normalized.rate * period as f64);
                println!("{:>7.3?} {:>2} / {}", rate, unit, period_name);
            }
        }
        Err(_) => {
            eprintln!("{PROG_NAME}: invalid input");
            exit(1);
        }
    }
}

fn nearest_power_of_1000_rate(mut bytes: f64) -> (f64, &'static str) {
    for unit in UNITS {
        if bytes < 1000.0 {
            return (bytes, unit);
        }
        bytes /= 1000.0;
    }

    return (0.0, "NA");
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Period {
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Year,
}

impl Period {
    fn as_seconds(&self) -> f64 {
        let seconds = match self {
            Period::Second => SECOND,
            Period::Minute => MINUTE,
            Period::Hour => HOUR,
            Period::Day => DAY,
            Period::Week => WEEK,
            Period::Month => MONTH,
            Period::Year => YEAR,
        };
        return seconds as f64;
    }
}

impl TryFrom<&str> for Period {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "s" | "sec" | "second" => Ok(Period::Second),
            "m" | "min" | "minute" => Ok(Period::Minute),
            "h" | "hr" | "hour" => Ok(Period::Hour),
            "d" | "day" => Ok(Period::Day),
            "w" | "wk" | "week" => Ok(Period::Week),
            "mon" | "month" => Ok(Period::Month),
            "y" | "yr" | "year" => Ok(Period::Year),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
enum ParseError {
    InvalidNumber,
    InvalidUnit,
    InvalidPeriod,
    UnexpectedCharacter,
}

#[derive(Debug)]
struct ParseResult {
    rate: f64,
    byte_multiplier: f64,
    period: Period,
}

impl ParseResult {
    fn normalized(&self) -> Self {
        let rate = self.rate * self.byte_multiplier / self.period.as_seconds();
        ParseResult {
            rate,
            byte_multiplier: 1.0,
            period: Period::Second,
        }
    }
}

fn parse(s: &str) -> Result<ParseResult, ParseError> {
    let mut p = Parser {
        buf: s.as_bytes(),
        pos: 0,
    };
    p.skip_whitespace();
    let rate: f64 = p.parse_number()?;
    p.skip_whitespace();
    let byte_multiplier: f64 = p.parse_unit()?;
    p.skip_whitespace();
    p.expect(b'/')?;
    p.skip_whitespace();
    let period: Period = p.parse_period()?;
    return Ok(ParseResult {
        rate,
        byte_multiplier,
        period,
    });
}

struct Parser<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> u8 {
        if self.pos >= self.buf.len() {
            return 0;
        }
        return self.buf[self.pos];
    }

    fn eof(&self) -> bool {
        return self.peek() == 0;
    }

    fn advance(&mut self) -> u8 {
        let b = self.peek();
        if b == 0 {
            return b;
        }
        self.pos += 1;
        return b;
    }

    fn expect(&mut self, expected: u8) -> Result<u8, ParseError> {
        let actual = self.advance();
        if actual == expected {
            return Ok(actual);
        }
        return Err(ParseError::UnexpectedCharacter);
    }

    fn skip_whitespace(&mut self) {
        while !self.eof() && self.peek().is_ascii_whitespace() {
            self.advance();
        }
    }

    fn parse_number(&mut self) -> Result<f64, ParseError> {
        let start_pos = self.pos;
        while !self.eof() && self.peek().is_ascii_digit() {
            self.advance();
        }
        // No digits
        // NB(vincent): shouldn't ever trigger, we get inside `parse_number`
        // because we saw a digit. Still gonna put it for good measure.
        if start_pos == self.pos {
            return Err(ParseError::InvalidNumber);
        }
        // No decimals
        if self.peek() == b'.' {
            self.advance(); // eat the '.'
            let decimals_start = self.pos;
            while !self.eof() && self.peek().is_ascii_digit() {
                self.advance();
            }
            if decimals_start == self.pos {
                return Err(ParseError::InvalidNumber);
            }
        }

        let s = unsafe { std::str::from_utf8_unchecked(&self.buf[start_pos..self.pos]) };
        match s.parse::<f64>() {
            Ok(x) => return Ok(x),
            Err(_) => return Err(ParseError::InvalidNumber),
        }
    }

    fn parse_unit(&mut self) -> Result<f64, ParseError> {
        let start_pos = self.pos;
        while !self.eof() && self.peek().is_ascii_alphabetic() {
            self.advance();
        }
        let unit = (&self.buf[start_pos..self.pos]).to_ascii_uppercase();
        for (i, candidate) in UNITS.iter().enumerate() {
            if candidate.as_bytes() == unit {
                return Ok(f64::powf(1000.0, i as f64));
            }
        }
        return Err(ParseError::InvalidUnit);
    }

    fn parse_period(&mut self) -> Result<Period, ParseError> {
        let start_pos = self.pos;
        while !self.eof() && self.peek().is_ascii_alphabetic() {
            self.advance();
        }
        let period = unsafe { std::str::from_utf8_unchecked(&self.buf[start_pos..self.pos]) };
        match Period::try_from(period) {
            Ok(p) => Ok(p),
            Err(()) => Err(ParseError::InvalidPeriod),
        }
    }
}

#[rustfmt::skip]
#[test]
fn test_parse_number() {
    let mut p = Parser { buf: b"", pos: 0 };
    assert!(p.parse_number().is_err());

    let mut p = Parser { buf: b"x", pos: 0 };
    assert!(p.parse_number().is_err());

    let mut p = Parser { buf: b"1", pos: 0 };
    assert_eq!(p.parse_number(), Ok(1.0));

    let mut p = Parser { buf: b"123", pos: 0 };
    assert_eq!(p.parse_number(), Ok(123.0));

    let mut p = Parser { buf: b"1.", pos: 0 };
    assert!(p.parse_number().is_err());

    let mut p = Parser { buf: b"1.25", pos: 0 };
    assert_eq!(p.parse_number(), Ok(1.25));
}

#[rustfmt::skip]
#[test]
fn test_parse_period() {
    use Period::*;
    let table: Vec<(Period, &[&str])> = vec![
        (Second, &["s", "sec", "second"]),
        (Minute, &["m", "min", "minute"]),
        (Hour, &["h", "hr", "hour"]),
        (Day, &["d", "day"]),
        (Week, &["w", "wk", "week"]),
        (Month, &["mon", "month"]),
        (Year, &["y", "yr", "year"]),
    ];
    for (period, string_reps) in table {
        for rep in string_reps.iter() {
            let mut p = Parser { buf: rep.as_bytes(), pos: 0 };
            assert_eq!(p.parse_period(), Ok(period));
        }
    }
}
