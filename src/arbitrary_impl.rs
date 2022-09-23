use crate::{Level, MetadataBuilder, RecordBuilder};
use arbitrary::{Arbitrary, Result, Unstructured};

impl<'a> Arbitrary<'a> for RecordBuilder<'a> {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        let target = &<&'a str>::arbitrary(u)?;
        let path = <&'a str>::arbitrary(u)?;
        let file = <&'a str>::arbitrary(u)?;

        let mut builder = RecordBuilder::new();

        builder
            // We can't yet provide an arbitrary fmt::Argument object because
            // the output of format_args! must be consumed where it is called.
            // It cannot be bound to a variable. See https://github.com/rust-lang/rust/issues/92698#ref-pullrequest-1225460272
            // .args(format_args!("{}", logoutput))
            .metadata(
                MetadataBuilder::new()
                    .level(Level::arbitrary(u)?)
                    .target(target)
                    .build(),
            )
            .file(Some(file.clone()))
            .line(Option::<u32>::arbitrary(u)?)
            .module_path(Some(path.clone()));

        return Ok(builder);
    }
}
#[cfg(test)]
mod tests {
    use crate::{logger, RecordBuilder};
    use arbitrary::{Arbitrary, Unstructured};

    #[derive(Arbitrary, Debug)]
    struct LogFuzzerInput<'a> {
        builder: RecordBuilder<'a>,
        message: String,
    }

    #[test]
    fn arbitrary_record() {
        let input: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        let mut buf = Unstructured::new(&input);
        let mut arb = LogFuzzerInput::arbitrary(&mut buf).unwrap();

        logger().log(&arb.builder.args(format_args!("{}", arb.message)).build());
    }
}
