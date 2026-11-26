use tracing::warn;
use zbus::{OwnedVariant, Variant};

pub fn value_to<'a, 'b, T>(value: &'a Variant<'b>) -> T
where
    T: std::default::Default + std::convert::TryFrom<&'a zbus::Variant<'b>>,
    <T as TryFrom<&'a Variant<'b>>>::Error: std::fmt::Debug,
{
    value
        .try_into()
        .inspect_err(|e| warn!("Convetion Error: {e:?}"))
        .unwrap_or_default()
}

pub fn value_some<'a, T>(value: Option<&'a OwnedVariant>) -> T
where
    T: std::default::Default + std::convert::TryFrom<&'a zbus::OwnedVariant>,
    <T as TryFrom<&'a OwnedVariant>>::Error: std::fmt::Debug,
{
    value
        .and_then(|v| {
            v.try_into()
                .inspect_err(|e| warn!("Convetion Error: {e:?}"))
                .ok()
        })
        .unwrap_or_default()
}

pub fn value_to_def<'a, T>(value: Option<&'a OwnedVariant>, def: T) -> T
where
    T: std::default::Default + std::convert::TryFrom<&'a zbus::OwnedVariant>,
    <T as TryFrom<&'a OwnedVariant>>::Error: std::fmt::Debug,
{
    value
        .and_then(|v| {
            v.try_into()
                .inspect_err(|e| warn!("Convertion Error: {e:?}"))
                .ok()
        })
        .unwrap_or(def)
}

pub fn get_array_str<'a>(value: &'a Variant<'a>) -> Vec<&'a str> {
    match value as &Variant {
        Variant::Array(array) => {
            // let v = array.inner();
            let mut vec = Vec::with_capacity(array.len());
            for mi in array.iter() {
                vec.push(value_to(mi));
            }
            vec
        }
        _ => {
            warn!("Wrong zvalue conversion: {:?}", value.value_signature());
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {

    use std::slice::Iter;

    use super::*;

    pub fn get_iter<'a>(value: &'a Variant<'a>) -> Iter<'a, Variant<'a>> {
        match value as &Variant {
            Variant::Array(array) => {
                // let v = array.inner();
                array.iter()
            }
            _ => {
                warn!("Wrong zvalue conversion: {:?}", value.value_signature());
                Iter::default()
            }
        }
    }

    #[test]
    fn test_var_str() {
        let slice = vec!["test1", "test2"];
        let v_vec = Variant::new(slice);

        for value in get_iter(&v_vec) {
            let s: &str = value.downcast_ref().unwrap_or_default();
            println!("{}", s)
        }
    }
}
