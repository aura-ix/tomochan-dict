use bincode::{Encode, Decode};
use crate::schema::{JsonParseable, get_str, parse_string_array, parse_single_or_multiple};

#[derive(Encode, Decode, Debug, Clone)]
pub struct TermMeta {
    pub term: String,
    pub mode: TermMetaMode,
    pub data: TermMetaData,
}

#[derive(Encode, Decode, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TermMetaMode {
    Freq = 0,
    Pitch = 1,
    Ipa = 2,
}

#[derive(Encode, Decode, Debug, Clone)]
pub enum TermMetaData {
    Frequency(FrequencyData),
    Pitch(PitchData),
    Ipa(IpaData),
}

#[derive(Encode, Decode, Debug, Clone)]
pub enum FrequencyData {
    Simple(Frequency),
    WithReading {
        reading: String,
        frequency: Frequency,
    },
}

#[derive(Encode, Decode, Debug, Clone)]
pub enum Frequency {
    Simple(FrequencyValue),
    Detailed {
        value: f32,
        display_value: Option<String>,
    },
}

#[derive(Encode, Decode, Debug, Clone)]
pub enum FrequencyValue {
    Number(f32),
    String(String),
}

#[derive(Encode, Decode, Debug, Clone)]
pub struct PitchData {
    pub reading: String,
    pub pitches: Vec<PitchAccent>,
}

#[derive(Encode, Decode, Debug, Clone)]
pub struct PitchAccent {
    pub position: PitchPosition,
    pub nasal: Option<NasalPositions>,
    pub devoice: Option<DevoicePositions>,
    pub tags: Vec<String>,
}

#[derive(Encode, Decode, Debug, Clone)]
pub enum PitchPosition {
    Numeric(u32),
    Pattern(String),
}

#[derive(Encode, Decode, Debug, Clone)]
pub enum NasalPositions {
    Single(u32),
    Multiple(Vec<u32>),
}

#[derive(Encode, Decode, Debug, Clone)]
pub enum DevoicePositions {
    Single(u32),
    Multiple(Vec<u32>),
}

#[derive(Encode, Decode, Debug, Clone)]
pub struct IpaData {
    pub reading: String,
    pub transcriptions: Vec<IpaTranscription>,
}

#[derive(Encode, Decode, Debug, Clone)]
pub struct IpaTranscription {
    pub ipa: String,
    pub tags: Vec<String>,
}

impl TermMeta {
    fn parse_frequency_data(value: &serde_json::Value) -> Result<FrequencyData, String> {
        if let Some(obj) = value.as_object() {
            if let (Some(reading), Some(freq)) = (obj.get("reading"), obj.get("frequency")) {
                return Ok(FrequencyData::WithReading {
                    reading: reading.as_str().ok_or("Invalid reading")?.to_string(),
                    frequency: Self::parse_frequency(freq)?,
                });
            }
        }
        Ok(FrequencyData::Simple(Self::parse_frequency(value)?))
    }
    
    fn parse_frequency(value: &serde_json::Value) -> Result<Frequency, String> {
        if let Some(obj) = value.as_object() {
            return Ok(Frequency::Detailed {
                value: obj.get("value").and_then(|v| v.as_f64()).ok_or("Missing frequency value")? as f32,
                display_value: obj.get("displayValue").and_then(|v| v.as_str()).map(String::from),
            });
        }
        
        if let Some(num) = value.as_f64() {
            return Ok(Frequency::Simple(FrequencyValue::Number(num as f32)));
        }
        
        if let Some(s) = value.as_str() {
            return Ok(Frequency::Simple(FrequencyValue::String(s.to_string())));
        }
        
        Err("Invalid frequency format".to_string())
    }
    
    fn parse_pitch_data(value: &serde_json::Value) -> Result<PitchData, String> {
        let obj = value.as_object().ok_or("Pitch data must be an object")?;
        
        Ok(PitchData {
            reading: obj.get("reading").and_then(|v| v.as_str()).ok_or("Missing reading")?.to_string(),
            pitches: obj.get("pitches")
                .and_then(|v| v.as_array())
                .ok_or("Missing pitches array")?
                .iter()
                .map(Self::parse_pitch_accent)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
    
    fn parse_pitch_accent(value: &serde_json::Value) -> Result<PitchAccent, String> {
        let obj = value.as_object().ok_or("Pitch accent must be an object")?;
        
        Ok(PitchAccent {
            position: Self::parse_pitch_position(obj.get("position").ok_or("Missing position")?)?,
            nasal: obj.get("nasal").map(Self::parse_nasal_positions).transpose()?,
            devoice: obj.get("devoice").map(Self::parse_devoice_positions).transpose()?,
            tags: obj.get("tags").map(parse_string_array).unwrap_or_default(),
        })
    }
    
    fn parse_pitch_position(value: &serde_json::Value) -> Result<PitchPosition, String> {
        if let Some(num) = value.as_u64() {
            return Ok(PitchPosition::Numeric(num as u32));
        }
        
        if let Some(s) = value.as_str() {
            return Ok(PitchPosition::Pattern(s.to_string()));
        }
        
        Err("Invalid pitch position".to_string())
    }
    
    fn parse_nasal_positions(value: &serde_json::Value) -> Result<NasalPositions, String> {
        parse_single_or_multiple(value, NasalPositions::Single, NasalPositions::Multiple)
    }
    
    fn parse_devoice_positions(value: &serde_json::Value) -> Result<DevoicePositions, String> {
        parse_single_or_multiple(value, DevoicePositions::Single, DevoicePositions::Multiple)
    }
    
    fn parse_ipa_data(value: &serde_json::Value) -> Result<IpaData, String> {
        let obj = value.as_object().ok_or("IPA data must be an object")?;
        
        Ok(IpaData {
            reading: obj.get("reading").and_then(|v| v.as_str()).ok_or("Missing reading")?.to_string(),
            transcriptions: obj.get("transcriptions")
                .and_then(|v| v.as_array())
                .ok_or("Missing transcriptions array")?
                .iter()
                .map(Self::parse_ipa_transcription)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
    
    fn parse_ipa_transcription(value: &serde_json::Value) -> Result<IpaTranscription, String> {
        let obj = value.as_object().ok_or("IPA transcription must be an object")?;
        
        Ok(IpaTranscription {
            ipa: obj.get("ipa").and_then(|v| v.as_str()).ok_or("Missing ipa")?.to_string(),
            tags: obj.get("tags").map(parse_string_array).unwrap_or_default(),
        })
    }
}

impl JsonParseable for TermMeta {
    fn from_json_array(arr: &[serde_json::Value]) -> Result<Self, String> {
        if arr.len() != 3 {
            return Err("Term meta array must have exactly 3 elements".to_string());
        }
        
        let term = get_str(&arr[0], "term")?;
        let mode_str = get_str(&arr[1], "mode")?;
        
        let (mode, data) = match mode_str.as_str() {
            "freq" => {
                let freq_data = Self::parse_frequency_data(&arr[2])?;
                (TermMetaMode::Freq, TermMetaData::Frequency(freq_data))
            }
            "pitch" => {
                let pitch_data = Self::parse_pitch_data(&arr[2])?;
                (TermMetaMode::Pitch, TermMetaData::Pitch(pitch_data))
            }
            "ipa" => {
                let ipa_data = Self::parse_ipa_data(&arr[2])?;
                (TermMetaMode::Ipa, TermMetaData::Ipa(ipa_data))
            }
            _ => return Err(format!("Unknown mode: {}", mode_str)),
        };
        
        Ok(TermMeta { term, mode, data })
    }
}