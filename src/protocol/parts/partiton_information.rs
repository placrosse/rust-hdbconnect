use crate::protocol::util;
use crate::{HdbError, HdbResult};

use byteorder::{LittleEndian, ReadBytesExt};
use std::io;

#[derive(Debug)]
pub struct PartitionInformation {
    partition_method: PartitionMethod,
    parameter_descriptor: Vec<ParameterDescriptor>,
    partitions: Vec<Partitions>,
}

#[derive(Debug, Clone, Copy)]
enum PartitionMethod {
    Invalid,
    RoundRobin,
    Hash,
}

impl PartitionMethod {
    pub fn from_i8(val: i8) -> HdbResult<PartitionMethod> {
        match val {
            0 => Ok(PartitionMethod::Invalid),
            1 => Ok(PartitionMethod::RoundRobin),
            2 => Ok(PartitionMethod::Hash),
            _ => Err(HdbError::Impl(format!(
                "PartitionMethod {} not implemented",
                val
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ParameterFunction {
    Invalid,
    Year,
    Month,
}

impl ParameterFunction {
    pub fn from_i8(val: i8) -> HdbResult<ParameterFunction> {
        match val {
            0 => Ok(ParameterFunction::Invalid),
            1 => Ok(ParameterFunction::Year),
            2 => Ok(ParameterFunction::Month),
            _ => Err(HdbError::Impl(format!(
                "ParameterFunction {} not implemented",
                val
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ParameterDescriptor {
    parameter_index: i32,
    parameter_function: ParameterFunction,
    attribute_type: i8,
}

#[derive(Debug, Clone, Copy)]
pub struct Partitions {
    val1: i32,
    val2: i32,
}

impl PartitionInformation {
    pub fn parse<T: io::BufRead>(rdr: &mut T) -> HdbResult<PartitionInformation> {
        let partition_method = PartitionMethod::from_i8(rdr.read_i8()?)?; // I1
        util::skip_bytes(7, rdr)?;
        let num_parameters = rdr.read_i32::<LittleEndian>()?;
        let num_partitions = rdr.read_i32::<LittleEndian>()?;
        let mut parameter_descriptor = vec![];
        for _ in 0..num_parameters {
            let desc = ParameterDescriptor {
                parameter_index: rdr.read_i32::<LittleEndian>()?,
                parameter_function: ParameterFunction::from_i8(rdr.read_i8()?)?,
                attribute_type: rdr.read_i8()?,
            };
            util::skip_bytes(2, rdr)?;
            parameter_descriptor.push(desc);
        }

        let mut partitions = vec![];

        // Missing in documentation, but it is 8 byte per partition
        // https://help.sap.com/viewer/7e4aba181371442d9e4395e7ff71b777/2.0.03/en-US/a6b5b33a790245efa06c67a781f80d15.html#loioeed44c1df1fc4f139079f36031b42ef1
        for _ in 0..num_partitions {
            partitions.push({
                Partitions {
                    val1: rdr.read_i32::<LittleEndian>()?,
                    val2: rdr.read_i32::<LittleEndian>()?,
                }
            });
        }

        Ok(PartitionInformation {
            partition_method,
            parameter_descriptor,
            partitions,
        })
    }
}
