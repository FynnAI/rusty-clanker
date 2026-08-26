//! M2-B02 Acceptance tests: the `ToNbtCompound`/`FromNbtCompound`/`NbtCompoundExt`
//! schema-conversion helper layer, exercised via a test-local example type (not a
//! deliverable of this blueprint).

use rc_nbt::{NbtPath, SchemaError, owned};

#[derive(Debug)]
struct ExamplePoint {
    x: i32,
    y: i32,
    label: String,
}

impl rc_nbt::ToNbtCompound for ExamplePoint {
    fn to_nbt_compound(&self) -> rc_nbt::owned::NbtCompound {
        rc_nbt::owned::NbtCompound::from_values(vec![
            ("x".into(), rc_nbt::owned::NbtTag::Int(self.x)),
            ("y".into(), rc_nbt::owned::NbtTag::Int(self.y)),
            (
                "label".into(),
                rc_nbt::owned::NbtTag::String(self.label.as_str().into()),
            ),
        ])
    }
}

impl rc_nbt::FromNbtCompound for ExamplePoint {
    fn from_nbt_compound<'a, 'tape>(
        compound: &rc_nbt::borrow::NbtCompound<'a, 'tape>,
        path: &rc_nbt::NbtPath,
    ) -> Result<Self, rc_nbt::SchemaError> {
        use rc_nbt::NbtCompoundExt;
        Ok(ExamplePoint {
            x: compound.require_int(path, "x")?,
            y: compound.require_int(path, "y")?,
            label: compound
                .require_string(path, "label")?
                .to_str()
                .into_owned(),
        })
    }
}

fn encode_and_decode(compound: owned::NbtCompound) -> Result<ExamplePoint, SchemaError> {
    use rc_nbt::FromNbtCompound;

    let root = owned::BaseNbt::new("", compound);
    let bytes = rc_nbt::write_owned(&root);
    let decoded = rc_nbt::read_borrowed(&bytes).unwrap();
    let base = match decoded {
        rc_nbt::borrow::Nbt::Some(base) => base,
        rc_nbt::borrow::Nbt::None => panic!("expected Nbt::Some"),
    };
    ExamplePoint::from_nbt_compound(&base.as_compound(), &NbtPath::root())
}

#[test]
fn round_trips_through_to_and_from_nbt_compound() {
    use rc_nbt::ToNbtCompound;

    let original = ExamplePoint {
        x: 3,
        y: -5,
        label: "hi".into(),
    };
    let compound = original.to_nbt_compound();
    let decoded = encode_and_decode(compound).unwrap();

    assert_eq!(decoded.x, original.x);
    assert_eq!(decoded.y, original.y);
    assert_eq!(decoded.label, original.label);
}

#[test]
fn missing_field_reports_exact_path_and_field_name() {
    let compound = owned::NbtCompound::from_values(vec![
        ("x".into(), owned::NbtTag::Int(3)),
        ("label".into(), owned::NbtTag::String("hi".into())),
    ]);

    let err = encode_and_decode(compound).unwrap_err();
    match err {
        SchemaError::MissingField { field, .. } => assert_eq!(field, "y"),
        other => panic!("expected MissingField, got {other:?}"),
    }
}

#[test]
fn wrong_type_reports_expected_and_actual_tag_id() {
    let compound = owned::NbtCompound::from_values(vec![
        ("x".into(), owned::NbtTag::String("not an int".into())),
        ("y".into(), owned::NbtTag::Int(-5)),
        ("label".into(), owned::NbtTag::String("hi".into())),
    ]);

    let err = encode_and_decode(compound).unwrap_err();
    match err {
        SchemaError::WrongType {
            field,
            expected,
            actual_id,
            ..
        } => {
            assert_eq!(field, "x");
            assert_eq!(expected, "Int");
            assert_eq!(actual_id, 8);
        }
        other => panic!("expected WrongType, got {other:?}"),
    }
}
