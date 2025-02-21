use std::{collections::HashMap, sync::OnceLock};

pub static KEYMAPS: OnceLock<HashMap<u8, KeyMap>> = OnceLock::new();
const AZERTY: &str = include_str!("../../../../assets/layouts/azerty.yml");
const QWERTY: &str = include_str!("../../../../assets/layouts/qwerty.yml");
const COMMON_REPEATED_KEYS: [&str; 4] = [" 󱊷 ", " 󰌑 ", " 󰁮 ", "  "];
pub fn init_keymaps() {
    let mut map = HashMap::<u8, KeyMap>::new();
    map.insert(
        Layout::Azerty as u8,
        serde_yaml::from_str::<KeyMap>(AZERTY).unwrap(),
    );
    map.insert(
        Layout::Qwerty as u8,
        serde_yaml::from_str::<KeyMap>(QWERTY).unwrap(),
    );
    KEYMAPS.set(map).expect("Error initializing KEYMAPS");
}

use anyhow::Error;
use serde::Deserialize;
use tamanoir_common::Layout;

use crate::app::AppResult;

#[derive(Deserialize, Debug)]
pub struct KeyMap {
    keys: HashMap<u8, String>,
    modifier: HashMap<u8, HashMap<u8, String>>,
}
impl KeyMap {
    pub fn get(&self, key_code: &u8, last_keycode: Option<&u8>) -> Vec<String> {
        let mut out = vec![];
        match last_keycode {
            None => {
                if let Some(key) = self.keys.get(key_code) {
                    out.push(key.to_string());
                }
            }
            Some(last_keycode) => match self.modifier.get(last_keycode) {
                Some(modifier_map) => {
                    if let Some(key) = modifier_map.get(key_code) {
                        out.push(key.to_string());
                    } else {
                        out.extend(self.get(last_keycode, None));
                        out.extend(self.get(key_code, None));
                    }
                }
                _ => {
                    out.extend(self.get(key_code, None));
                }
            },
        }
        out
    }
    pub fn is_modifier(&self, key_code: Option<&u8>) -> bool {
        if let Some(key_code) = key_code {
            return self.modifier.contains_key(key_code);
        }
        false
    }
}

pub fn parse_keycodes(key_codes: &Vec<u32>, layout: Layout) -> AppResult<Vec<String>> {
    let mut parsed_keys: Vec<String> = vec![];

    let key_map = KEYMAPS
        .get()
        .ok_or(Error::msg("error geting LAYOUT KEYMAPS"))?
        .get(&(layout as u8))
        .ok_or(Error::msg("unknow layout"))?;
    let key_codes: Result<Vec<u8>, _> = key_codes.iter().map(|kc| u8::try_from(*kc)).collect();
    let key_codes = key_codes.map_err(|_| "Couldn't parse keycodes")?;
    let mut previous_kc: Option<&u8> = None;
    for kc in &key_codes {
        if key_map.is_modifier(previous_kc) {
            let _ = parsed_keys.pop();
        }
        let mapped_keys = key_map.get(kc, previous_kc);

        parsed_keys.extend(mapped_keys);
        previous_kc = Some(kc);
    }
    Ok(parsed_keys)
}

pub fn format_keys(keycodes: Vec<String>) -> String {
    let mut fmt_keys: Vec<String> = vec![];
    let mut repeat_counter = 1;
    let mut last_key: Option<String> = None;
    for current_key in keycodes.into_iter() {
        if let Some(ref prev_key) = last_key {
            if current_key == *prev_key && COMMON_REPEATED_KEYS.contains(&current_key.as_str()) {
                repeat_counter += 1;
            } else {
                if repeat_counter > 1 {
                    fmt_keys.push(format!("(x{}) ", repeat_counter));
                }
                fmt_keys.push(current_key.clone());
                last_key = Some(current_key);
                repeat_counter = 1;
            }
        } else {
            fmt_keys.push(current_key.clone());
            last_key = Some(current_key);
        }
    }
    if repeat_counter > 1 {
        fmt_keys.push(format!("(x{}) ", repeat_counter))
    }
    fmt_keys.join("")
}
