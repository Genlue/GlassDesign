use glass_design::prelude::*;
use js_sys::{Array, Object, Reflect};
use wasm_bindgen::{JsCast, JsValue};

pub(crate) fn error(message: impl AsRef<str>) -> JsValue {
    js_sys::TypeError::new(message.as_ref()).into()
}

pub(crate) struct Props<'a> {
    value: &'a JsValue,
    path: String,
}

impl<'a> Props<'a> {
    pub fn new(value: &'a JsValue, path: impl Into<String>) -> Result<Self, JsValue> {
        let path = path.into();
        if !value.is_object() || Array::is_array(value) {
            return Err(error(format!("{path} must be an object")));
        }
        Ok(Self { value, path })
    }

    pub fn allow_only(&self, keys: &[&str]) -> Result<(), JsValue> {
        for key in Object::keys(self.value.unchecked_ref::<Object>()) {
            if let Some(key) = key.as_string().filter(|key| !keys.contains(&key.as_str())) {
                return Err(error(format!("{}.{key} is not a reactive prop", self.path)));
            }
        }
        Ok(())
    }

    fn bad(&self, name: &str, expected: &str) -> JsValue {
        error(format!("{}.{} must be {expected}", self.path, name))
    }

    fn get(&self, name: &str) -> Result<Option<JsValue>, JsValue> {
        let value = Reflect::get(self.value, &JsValue::from_str(name))?;
        Ok((!value.is_undefined() && !value.is_null()).then_some(value))
    }

    pub fn optional_text(&self, name: &str) -> Result<Option<String>, JsValue> {
        self.get(name)?
            .map(|v| v.as_string().ok_or_else(|| self.bad(name, "a string")))
            .transpose()
    }

    pub fn text(&self, name: &str) -> Result<String, JsValue> {
        Ok(self.optional_text(name)?.unwrap_or_default())
    }

    pub fn required_text(&self, name: &str) -> Result<String, JsValue> {
        self.optional_text(name)?
            .ok_or_else(|| self.bad(name, "a string"))
    }

    pub fn optional_flag(&self, name: &str) -> Result<Option<bool>, JsValue> {
        self.get(name)?
            .map(|v| v.as_bool().ok_or_else(|| self.bad(name, "a boolean")))
            .transpose()
    }

    pub fn flag(&self, name: &str) -> Result<bool, JsValue> {
        Ok(self.optional_flag(name)?.unwrap_or(false))
    }

    pub fn uint(&self, name: &str) -> Result<Option<u32>, JsValue> {
        self.get(name)?
            .map(|v| {
                v.as_f64()
                    .filter(|n| *n >= 0.0 && *n <= u32::MAX.into() && n.fract() == 0.0)
                    .map(|n| n as u32)
                    .ok_or_else(|| self.bad(name, "a non-negative 32-bit integer"))
            })
            .transpose()
    }

    pub fn list<T>(
        &self,
        name: &str,
        parse: impl Fn(&Props<'_>) -> Result<T, JsValue>,
    ) -> Result<Vec<T>, JsValue> {
        let Some(value) = self.get(name)? else {
            return Err(self.bad(name, "an array"));
        };
        if !Array::is_array(&value) {
            return Err(self.bad(name, "an array"));
        }
        let array = Array::from(&value);
        (0..array.length())
            .map(|i| {
                parse(&Props::new(
                    &array.get(i),
                    format!("{}.{name}[{i}]", self.path),
                )?)
            })
            .collect()
    }

    pub fn icon(&self) -> Result<Option<IconName>, JsValue> {
        self.optional_text("icon")?
            .filter(|key| !key.is_empty())
            .map(|key| {
                IconName::lookup(&key)
                    .ok_or_else(|| self.bad("icon", &format!("a built-in icon key (got {key:?})")))
            })
            .transpose()
    }

    pub fn size(&self) -> Result<Size, JsValue> {
        self.choice(
            "size",
            Size::Md,
            &[
                ("sm", Size::Sm),
                ("small", Size::Sm),
                ("md", Size::Md),
                ("medium", Size::Md),
                ("lg", Size::Lg),
                ("large", Size::Lg),
            ],
        )
    }

    pub fn choice<T: Copy>(
        &self,
        name: &str,
        default: T,
        choices: &[(&str, T)],
    ) -> Result<T, JsValue> {
        let Some(key) = self.optional_text(name)? else {
            return Ok(default);
        };
        choices
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| *v)
            .ok_or_else(|| {
                let keys: Vec<_> = choices.iter().map(|(k, _)| *k).collect();
                self.bad(name, &format!("one of {} (got {key:?})", keys.join(", ")))
            })
    }
}
