//! The closed intrinsic unit vocabulary and exact canonical conversion factors.

use core::num::NonZero;

use aldebaran::{
    ice::Ice,
    prelude::Report,
    report::annotated::{InlineAnnotations, label::Label},
};
use fack::prelude::Error;

use crate::{
    dimension::{Axis, Dimension, Tag},
    domain::Count,
    number::Number,
    quantity::Quantity,
    source::Origin,
};

/// A conversion selected from Verte's intrinsic unit symbols.
#[derive(Debug, Clone)]
// NOTE(invariant): The scale and conversion kind come from the closed unit vocabulary.
pub struct Unit {
    /// Conversion behavior for the selected unit.
    kind: Kind,

    /// Exact multiplicative conversion to canonical units.
    scale: Number,
}

/// Fixed semantic conversion categories, including affine and nominal units.
#[derive(Debug, Clone)]
enum Kind {
    /// A linear physical dimension.
    Linear([i8; 8]),

    /// A tagged dimension whose scale has no implicit conversion.
    Tagged(Tag),

    /// Symbols per second, distinct from ordinary frequency.
    Symbols,

    /// Absolute kelvin.
    Kelvin,

    /// Celsius with its exact offset from kelvin.
    Celsius,

    /// Logarithmic decibel gain.
    Gain,

    /// A source-declared count, optionally measured per second.
    Count {
        /// Required unit symbol in declarations.
        name: &'static str,

        /// Whether the unit measures a count rate.
        rate: bool,
    },
}

impl Unit {
    /// Resolve an exact case-sensitive suffix without registering host-defined units.
    #[expect(
        clippy::missing_inline_in_public_items,
        reason = "The intrinsic unit table is kept out of line."
    )]
    pub fn lookup(suffix: &str) -> Option<Self> {
        let (kind, numerator, denominator): (Kind, u64, u64) = match suffix {
            "bit" => (Kind::Linear([0, 0, 0, 0, 0, 0, 0, 1]), 1, 1),
            "B" => (Kind::Linear([0, 0, 0, 0, 0, 0, 0, 1]), 8, 1),
            "kB" => (Kind::Linear([0, 0, 0, 0, 0, 0, 0, 1]), 8000, 1),
            "MB" => (Kind::Linear([0, 0, 0, 0, 0, 0, 0, 1]), 8000000, 1),
            "GB" => (Kind::Linear([0, 0, 0, 0, 0, 0, 0, 1]), 8000000000, 1),
            "KiB" => (Kind::Linear([0, 0, 0, 0, 0, 0, 0, 1]), 8192, 1),
            "MiB" => (Kind::Linear([0, 0, 0, 0, 0, 0, 0, 1]), 0x0080_0000, 1),
            "GiB" => (Kind::Linear([0, 0, 0, 0, 0, 0, 0, 1]), 0x0002_0000_0000, 1),
            "TiB" => (Kind::Linear([0, 0, 0, 0, 0, 0, 0, 1]), 0x0800_0000_0000, 1),
            "ps" => (Kind::Linear([1, 0, 0, 0, 0, 0, 0, 0]), 1, 1000000000000),
            "ns" => (Kind::Linear([1, 0, 0, 0, 0, 0, 0, 0]), 1, 1000000000),
            "us" => (Kind::Linear([1, 0, 0, 0, 0, 0, 0, 0]), 1, 1000000),
            "ms" => (Kind::Linear([1, 0, 0, 0, 0, 0, 0, 0]), 1, 1000),
            "s" => (Kind::Linear([1, 0, 0, 0, 0, 0, 0, 0]), 1, 1),
            "min" => (Kind::Linear([1, 0, 0, 0, 0, 0, 0, 0]), 60, 1),
            "h" => (Kind::Linear([1, 0, 0, 0, 0, 0, 0, 0]), 3600, 1),
            "Hz" => (Kind::Linear([-1, 0, 0, 0, 0, 0, 0, 0]), 1, 1),
            "kHz" => (Kind::Linear([-1, 0, 0, 0, 0, 0, 0, 0]), 1000, 1),
            "MHz" => (Kind::Linear([-1, 0, 0, 0, 0, 0, 0, 0]), 1000000, 1),
            "GHz" => (Kind::Linear([-1, 0, 0, 0, 0, 0, 0, 0]), 1000000000, 1),
            "bps" => (Kind::Linear([-1, 0, 0, 0, 0, 0, 0, 1]), 1, 1),
            "kbps" => (Kind::Linear([-1, 0, 0, 0, 0, 0, 0, 1]), 1000, 1),
            "Mbps" => (Kind::Linear([-1, 0, 0, 0, 0, 0, 0, 1]), 1000000, 1),
            "Gbps" => (Kind::Linear([-1, 0, 0, 0, 0, 0, 0, 1]), 1000000000, 1),
            "kBps" => (Kind::Linear([-1, 0, 0, 0, 0, 0, 0, 1]), 8000, 1),
            "MBps" => (Kind::Linear([-1, 0, 0, 0, 0, 0, 0, 1]), 8000000, 1),
            "uV" => (Kind::Linear([-3, 2, 1, -1, 0, 0, 0, 0]), 1, 1000000),
            "mV" => (Kind::Linear([-3, 2, 1, -1, 0, 0, 0, 0]), 1, 1000),
            "V" => (Kind::Linear([-3, 2, 1, -1, 0, 0, 0, 0]), 1, 1),
            "uohm" => (Kind::Linear([-3, 2, 1, -2, 0, 0, 0, 0]), 1, 1000000),
            "mohm" => (Kind::Linear([-3, 2, 1, -2, 0, 0, 0, 0]), 1, 1000),
            "ohm" => (Kind::Linear([-3, 2, 1, -2, 0, 0, 0, 0]), 1, 1),
            "uW" => (Kind::Linear([-3, 2, 1, 0, 0, 0, 0, 0]), 1, 1000000),
            "mW" => (Kind::Linear([-3, 2, 1, 0, 0, 0, 0, 0]), 1, 1000),
            "W" => (Kind::Linear([-3, 2, 1, 0, 0, 0, 0, 0]), 1, 1),
            "pA" => (Kind::Linear([0, 0, 0, 1, 0, 0, 0, 0]), 1, 1000000000000),
            "nA" => (Kind::Linear([0, 0, 0, 1, 0, 0, 0, 0]), 1, 1000000000),
            "uA" => (Kind::Linear([0, 0, 0, 1, 0, 0, 0, 0]), 1, 1000000),
            "mA" => (Kind::Linear([0, 0, 0, 1, 0, 0, 0, 0]), 1, 1000),
            "A" => (Kind::Linear([0, 0, 0, 1, 0, 0, 0, 0]), 1, 1),
            "uWh" => (Kind::Linear([-2, 2, 1, 0, 0, 0, 0, 0]), 3600, 1000000),
            "mWh" => (Kind::Linear([-2, 2, 1, 0, 0, 0, 0, 0]), 3600, 1000),
            "Wh" => (Kind::Linear([-2, 2, 1, 0, 0, 0, 0, 0]), 3600, 1),
            "J" => (Kind::Linear([-2, 2, 1, 0, 0, 0, 0, 0]), 1, 1),
            "uAh" => (Kind::Linear([1, 0, 0, 1, 0, 0, 0, 0]), 3600, 1000000),
            "mAh" => (Kind::Linear([1, 0, 0, 1, 0, 0, 0, 0]), 3600, 1000),
            "Ah" => (Kind::Linear([1, 0, 0, 1, 0, 0, 0, 0]), 3600, 1),
            "Coulomb" => (Kind::Linear([1, 0, 0, 1, 0, 0, 0, 0]), 1, 1),
            "fF" => (Kind::Linear([4, -2, -1, 2, 0, 0, 0, 0]), 1, 1000000000000000),
            "pF" => (Kind::Linear([4, -2, -1, 2, 0, 0, 0, 0]), 1, 1000000000000),
            "nF" => (Kind::Linear([4, -2, -1, 2, 0, 0, 0, 0]), 1, 1000000000),
            "uF" => (Kind::Linear([4, -2, -1, 2, 0, 0, 0, 0]), 1, 1000000),
            "F" => (Kind::Linear([4, -2, -1, 2, 0, 0, 0, 0]), 1, 1),
            "dC" => (Kind::Linear([0, 0, 0, 0, 1, 0, 0, 0]), 1, 1),
            "dK" => (Kind::Linear([0, 0, 0, 0, 1, 0, 0, 0]), 1, 1),
            "um" => (Kind::Linear([0, 1, 0, 0, 0, 0, 0, 0]), 1, 1000000),
            "mm" => (Kind::Linear([0, 1, 0, 0, 0, 0, 0, 0]), 1, 1000),
            "cm" => (Kind::Linear([0, 1, 0, 0, 0, 0, 0, 0]), 1, 100),
            "m" => (Kind::Linear([0, 1, 0, 0, 0, 0, 0, 0]), 1, 1),
            "km" => (Kind::Linear([0, 1, 0, 0, 0, 0, 0, 0]), 1000, 1),
            "Pa" => (Kind::Linear([-2, -1, 1, 0, 0, 0, 0, 0]), 1, 1),
            "kPa" => (Kind::Linear([-2, -1, 1, 0, 0, 0, 0, 0]), 1000, 1),
            "MPa" => (Kind::Linear([-2, -1, 1, 0, 0, 0, 0, 0]), 1000000, 1),
            "percent" => (Kind::Linear([0, 0, 0, 0, 0, 0, 0, 0]), 1, 100),
            "bp" => (Kind::Linear([0, 0, 0, 0, 0, 0, 0, 0]), 1, 10000),
            "ppm" => (Kind::Linear([0, 0, 0, 0, 0, 0, 0, 0]), 1, 1000000),
            "lux" => (Kind::Linear([0, -2, 0, 0, 0, 0, 1, 0]), 1, 1),
            "K" => (Kind::Kelvin, 1, 1),
            "C" => (Kind::Celsius, 1, 1),
            "mC" => (Kind::Celsius, 1, 1000),
            "dB" => (Kind::Gain, 1, 1),
            "deg" => (Kind::Tagged(Tag::Degree), 1, 1),
            "rad" => (Kind::Tagged(Tag::Radian), 1, 1),
            "Bd" | "kBd" | "MBd" => {
                let numerator = match suffix {
                    "kBd" => 1000,
                    "MBd" => 1_000_000,
                    _ => 1,
                };

                (Kind::Symbols, numerator, 1)
            },
            "page" => (
                Kind::Count {
                    name: "page",
                    rate: false,
                },
                1,
                1,
            ),
            "sector" => (
                Kind::Count {
                    name: "sector",
                    rate: false,
                },
                1,
                1,
            ),
            "block" => (
                Kind::Count {
                    name: "block",
                    rate: false,
                },
                1,
                1,
            ),
            "cacheline" => (
                Kind::Count {
                    name: "cacheline",
                    rate: false,
                },
                1,
                1,
            ),
            "cycle" => (
                Kind::Count {
                    name: "cycle",
                    rate: false,
                },
                1,
                1,
            ),
            "tick" => (
                Kind::Count {
                    name: "tick",
                    rate: false,
                },
                1,
                1,
            ),
            "frame" => (
                Kind::Count {
                    name: "frame",
                    rate: false,
                },
                1,
                1,
            ),
            "sample" => (
                Kind::Count {
                    name: "sample",
                    rate: false,
                },
                1,
                1,
            ),
            "px" => (
                Kind::Count {
                    name: "pixel",
                    rate: false,
                },
                1,
                1,
            ),
            "fps" => (
                Kind::Count {
                    name: "frame",
                    rate: true,
                },
                1,
                1,
            ),
            "sps" => (
                Kind::Count {
                    name: "sample",
                    rate: true,
                },
                1,
                1,
            ),
            "rpm" => (
                Kind::Count {
                    name: "revolution",
                    rate: true,
                },
                1,
                60,
            ),
            _ => return None,
        };

        let denominator = Ice::some(NonZero::new(denominator));
        let scale = Number::fraction(numerator, denominator);

        Some(Self { kind, scale })
    }

    /// Retrieve the count declaration required by this suffix, when applicable.
    #[inline]
    pub const fn domain(&self) -> Option<&'static str> {
        let Self { kind, .. } = self;

        match kind {
            Kind::Count { name, .. } => Some(name),
            _ => None,
        }
    }

    /// Convert an exact magnitude after resolving any required count declaration.
    #[inline]
    pub fn apply(&self, magnitude: &Number, domain: Option<&Count>, origin: Origin) -> Result<Quantity, CountBinding> {
        let Self { kind, scale } = self;

        let magnitude = magnitude.multiply(scale);
        let quantity = match kind {
            Kind::Linear(exponents) => {
                let dimension = Dimension::physical(exponents.map(Into::into));

                Quantity::linear(magnitude, dimension)
            },
            Kind::Tagged(tag) => Quantity::linear(magnitude, Dimension::tag(tag.clone())),
            Kind::Symbols => {
                let dimension = Dimension::tag(Tag::Symbol).divide(&Dimension::axis(Axis::Time));

                Quantity::linear(magnitude, dimension)
            },
            Kind::Kelvin => Quantity::temperature(magnitude),
            Kind::Celsius => {
                let offset = Number::fraction(27_315, Ice::some(NonZero::new(100)));
                let magnitude = magnitude.add(&offset);

                Quantity::temperature(magnitude)
            },
            Kind::Gain => Quantity::gain(magnitude),
            Kind::Count { name, rate } => match domain {
                Some(domain) if domain.name() == *name => {
                    let dimension = Dimension::tag(Tag::Count(domain.origin().clone()));
                    let dimension = if *rate {
                        dimension.divide(&Dimension::axis(Axis::Time))
                    } else {
                        dimension
                    };

                    Quantity::linear(magnitude, dimension)
                },
                _ => {
                    let labels = InlineAnnotations::new(
                        Label::new(
                            "this unit requires its matching source-declared count domain",
                            origin.span(),
                        ),
                        [],
                    );

                    return Err(CountBinding {
                        origin,
                        required: name,
                        labels,
                    });
                },
            },
        };

        Ok(quantity)
    }
}

/// A nominal suffix lacks its corresponding count declaration.
#[derive(Debug, Clone, Report, Error)]
#[error(inline)]
#[error("unit requires a matching count domain")]
#[report(title = "missing or mismatched count domain", annotations = labels)]
// NOTE(invariant): The label uses the byte coordinates qualified by `origin`.
pub struct CountBinding {
    /// Source occurrence of the quantity.
    origin: Origin,

    /// Declaration name required by the intrinsic suffix.
    required: &'static str,

    /// Primary quantity location.
    labels: InlineAnnotations<Label<&'static str>, 0>,
}

impl CountBinding {
    /// Retrieve the quantity's canonical source location.
    #[inline]
    pub const fn origin(&self) -> &Origin {
        let Self { origin, .. } = self;

        origin
    }

    /// Retrieve the missing or mismatched declaration name.
    #[inline]
    pub const fn required(&self) -> &'static str {
        let &Self { required, .. } = self;

        required
    }
}

#[cfg(test)]
mod tests {
    use core::num::NonZero;

    use aldebaran::prelude::Span;

    use super::Unit;
    use crate::{
        domain::Count,
        number::Number,
        source::{Id, Origin},
    };

    #[test]
    fn normative_unit_equalities_use_exact_scales_and_affine_offsets() {
        let origin = Origin::new(Id::new("test"), Span::unit(0));

        for (left, left_value, right, right_value) in [
            ("kB", 1, "B", 1000),
            ("KiB", 1, "B", 1024),
            ("MB", 1, "B", 1_000_000),
            ("MiB", 1, "B", 0x0010_0000),
            ("B", 1, "bit", 8),
            ("Wh", 1, "J", 3600),
            ("Ah", 1, "Coulomb", 3600),
            ("C", 25, "mC", 25_000),
            ("dC", 1, "dK", 1),
            ("percent", 100, "ppm", 1_000_000),
        ] {
            let left = Unit::lookup(left)
                .expect("specified unit")
                .apply(&Number::integer(left_value), None, origin.clone())
                .expect("intrinsic unit");
            let right = Unit::lookup(right)
                .expect("specified unit")
                .apply(&Number::integer(right_value), None, origin.clone())
                .expect("intrinsic unit");

            assert_eq!(left, right);
        }

        let kelvin = Number::fraction(29_815, NonZero::new(100).expect("nonzero"));
        let celsius = Unit::lookup("C")
            .expect("Celsius")
            .apply(&Number::integer(25), None, origin.clone())
            .expect("temperature");
        let kelvin = Unit::lookup("K")
            .expect("kelvin")
            .apply(&kelvin, None, origin.clone())
            .expect("temperature");
        let volts = Unit::lookup("mV")
            .expect("millivolts")
            .apply(&Number::integer(1), None, origin)
            .expect("voltage");
        let fraction = Number::fraction(1, NonZero::new(1000).expect("nonzero"));

        assert_eq!(celsius, kelvin);
        assert_eq!(volts.magnitude(), &fraction);
    }

    #[test]
    fn rates_preserve_information_and_symbol_semantics() {
        let origin = Origin::new(Id::new("test"), Span::unit(0));
        let size = Unit::lookup("kB")
            .expect("kilobytes")
            .apply(&Number::integer(1), None, origin.clone())
            .expect("size");
        let second = Unit::lookup("s")
            .expect("seconds")
            .apply(&Number::integer(1), None, origin.clone())
            .expect("duration");
        let rate = Unit::lookup("kbps")
            .expect("kilobits per second")
            .apply(&Number::integer(8), None, origin.clone())
            .expect("rate");
        let frequency = Unit::lookup("Hz")
            .expect("frequency")
            .apply(&Number::integer(1), None, origin.clone())
            .expect("frequency");
        let baud = Unit::lookup("Bd")
            .expect("symbol rate")
            .apply(&Number::integer(1), None, origin.clone())
            .expect("symbol rate");

        assert_eq!(size.divide(&second, origin).expect("rate"), rate);
        assert_ne!(frequency, baud);
        assert!(Unit::lookup("kB/s").is_none());
        assert!(Unit::lookup("KIB").is_none());
        assert!(Unit::lookup("ohm").is_some());
        assert!(Unit::lookup("Ohm").is_none());
    }

    #[test]
    fn nominal_units_require_the_matching_source_declaration() {
        let origin = Origin::new(Id::new("units"), Span::unit(0));
        let page = Count::new(origin.clone(), "page");
        let other_page = Count::new(Origin::new(Id::new("other"), Span::unit(0)), "page");
        let sector = Count::new(origin.clone(), "sector");
        let unit = Unit::lookup("page").expect("page suffix");
        let value = Number::integer(16);

        assert_eq!(unit.domain(), Some("page"));
        assert_eq!(
            unit.apply(&value, None, origin.clone())
                .expect_err("missing domain")
                .required(),
            "page"
        );
        assert!(unit.apply(&value, Some(&sector), origin.clone()).is_err());
        assert_ne!(
            unit.apply(&value, Some(&page), origin.clone()).expect("page count"),
            unit.apply(&value, Some(&other_page), origin).expect("other count")
        );
    }
}
