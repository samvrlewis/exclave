extern crate systemd_parser;

use unit::UnitName;
use std::path::Path;
use std::io;
use std::io::Read;
use std::fs::File;
use std::fmt;
use self::systemd_parser::items::DirectiveEntry;
use self::systemd_parser::errors::ParserError;

pub enum JigIncompatibleReason {
    TestProgramReturnedNonzero(String, i32),
    TestProgramFailed(String),
    TestFileNotPresent(String),
}

impl fmt::Display for JigIncompatibleReason {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &JigIncompatibleReason::TestProgramFailed(ref program_name) => write!(f, "Test program {} failed", program_name),
            &JigIncompatibleReason::TestProgramReturnedNonzero(ref program_name, val) => write!(f, "Test program {} returned {}", program_name, val),
            &JigIncompatibleReason::TestFileNotPresent(ref file_name) => write!(f, "Test file {} not present", file_name),
        }
    }
}

pub enum JigDescriptionError {
    InvalidUnitName,
    MissingJigSection,
    FileOpenError(io::Error),
    ParseError(ParserError),
}

impl From<io::Error> for JigDescriptionError {
    fn from(error: io::Error) -> Self {
        JigDescriptionError::FileOpenError(error)
    }
}

impl From<self::systemd_parser::errors::ParserError> for JigDescriptionError {
    fn from(error: self::systemd_parser::errors::ParserError) -> Self {
        JigDescriptionError::ParseError(error)
    }
}

impl fmt::Display for JigDescriptionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &JigDescriptionError::InvalidUnitName => write!(f, "Invalid jig unit name"),
            &JigDescriptionError::MissingJigSection => write!(f, "Missing [Jig] section"),
            &JigDescriptionError::FileOpenError(ref e) => {
                write!(f, "Unable to open .jig file: {}", e)
            }
            &JigDescriptionError::ParseError(ref e) => {
                write!(f, "Parse error reading .jig file: {}", e)
            }
        }
    }
}


/// A struct defining an in-memory representation of a .jig file
pub struct JigDescription {
    /// The id of the unit (including the kind)
    id: UnitName,

    /// A short name
    name: String,

    /// A detailed description of this jig, up to one paragraph.
    description: String,

    /// Name of the scenario to run by default, if any
    default_scenario: Option<String>,

    /// The default directory for programs on this jig, if any
    working_directory: Option<String>,

    /// A program to run to determine if this jig is compatible, if any
    test_program: Option<String>,

    /// A file whose existence indicates this jig is compatible
    test_file: Option<String>,
}

impl JigDescription {
    pub fn from_path(path: &Path) -> Result<JigDescription, JigDescriptionError> {
        let unit_name = match UnitName::from_path(path) {
            Some(name) => name,
            None => return Err(JigDescriptionError::InvalidUnitName),
        };

        // Parse the file into a systemd unit_file object
        let mut contents = String::with_capacity(8192);
        File::open(path)?.read_to_string(&mut contents)?;
        let unit_file = systemd_parser::parse_string(&contents)?;

        if !unit_file.has_category("Jig") {
            return Err(JigDescriptionError::MissingJigSection);
        }

        let mut jig_description = JigDescription {
            id: unit_name,
            name: "".to_owned(),
            description: "".to_owned(),
            default_scenario: None,
            working_directory: None,
            test_program: None,
            test_file: None,
        };

        for entry in unit_file.lookup_by_category("Jig") {
            match entry {
                &DirectiveEntry::Solo(ref directive) => {
                    match directive.key() {
                        "Name" => jig_description.name = directive.value().unwrap_or("").to_owned(),
                        "Description" => {
                            jig_description.description = directive.value().unwrap_or("").to_owned()
                        }
                        "TestFile" => {
                            jig_description.test_file = match directive.value() {
                                Some(s) => Some(s.to_owned()),
                                None => None,
                            }
                        }
                        "TestProgram" => {
                            jig_description.test_program = match directive.value() {
                                Some(s) => Some(s.to_owned()),
                                None => None,
                            }
                        }
                        &_ => (),
                    }
                }
                &_ => (),
            }
        }
        Ok(jig_description)
    }

    /// Determine if a unit is compatible with this system.
    /// Returns Ok(()) if it is, and Err(String) if not.
    pub fn is_compatible(&self) -> Result<(), JigIncompatibleReason> {
        if let Some(ref test_file) = self.test_file {
            if !Path::new(&test_file).exists() {
                return Err(JigIncompatibleReason::TestFileNotPresent(test_file.clone()));
            }
        }
        Ok(())
    }
}