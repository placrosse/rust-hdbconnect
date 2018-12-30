use super::part::Parts;
use super::part_attributes::PartAttributes;
use super::partkind::PartKind;
use super::parts::authfields::AuthFields;
use super::parts::client_info::ClientInfo;
use super::parts::connect_options::ConnectOptions;
use super::parts::execution_result::ExecutionResult;
use super::parts::output_parameters::OutputParameters;
use super::parts::parameter_descriptor::ParameterDescriptor;
use super::parts::parameters::Parameters;
use super::parts::partiton_information::PartitionInformation;
use super::parts::read_lob_reply::ReadLobReply;
use super::parts::resultset::ResultSet;
use super::parts::resultset_metadata::ResultSetMetadata;
use super::parts::server_error::ServerError;
use super::parts::statement_context::StatementContext;
use super::parts::topology::Topology;
use super::parts::transactionflags::TransactionFlags;
use super::parts::xat_options::XatOptions;
use crate::conn_core::AmConnCore;
use crate::protocol::parts::client_context::ClientContext;
use crate::protocol::parts::command_info::CommandInfo;
use crate::protocol::parts::commit_options::CommitOptions;
use crate::protocol::parts::fetch_options::FetchOptions;
use crate::protocol::parts::lob_flags::LobFlags;
use crate::protocol::parts::read_lob_request::ReadLobRequest;
use crate::protocol::parts::session_context::SessionContext;
use crate::{HdbError, HdbResult};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use cesu8;
use std::io;

#[derive(Debug)]
pub(crate) enum Argument<'a> {
    Auth(AuthFields),
    ClientContext(ClientContext),
    ClientInfo(ClientInfo),
    Command(&'a str),
    CommandInfo(CommandInfo),
    CommitOptions(CommitOptions),
    ConnectOptions(ConnectOptions),
    Error(Vec<ServerError>),
    FetchOptions(FetchOptions),
    FetchSize(u32),
    LobFlags(LobFlags),
    OutputParameters(OutputParameters),
    ParameterMetadata(Vec<ParameterDescriptor>),
    Parameters(Parameters),
    ReadLobRequest(ReadLobRequest),
    ReadLobReply(ReadLobReply),
    ResultSet(Option<ResultSet>),
    ResultSetId(u64),
    ResultSetMetadata(ResultSetMetadata),
    ExecutionResult(Vec<ExecutionResult>),
    SessionContext(SessionContext),
    StatementContext(StatementContext),
    StatementId(u64),
    PartitionInformation(PartitionInformation),
    TableLocation(Vec<i32>),
    TopologyInformation(Topology),
    TransactionFlags(TransactionFlags),
    XatOptions(XatOptions),
}

impl<'a> Argument<'a> {
    // only called on output (serialize)
    pub fn count(&self) -> HdbResult<usize> {
        Ok(match *self {
            Argument::Auth(_)
            | Argument::ClientContext(_)
            | Argument::Command(_)
            | Argument::FetchSize(_)
            | Argument::ResultSetId(_)
            | Argument::StatementId(_)
            | Argument::TopologyInformation(_)
            | Argument::ReadLobRequest(_) => 1,
            Argument::ClientInfo(ref client_info) => client_info.count(),
            Argument::CommandInfo(ref opts) => opts.count(),
            Argument::CommitOptions(ref opts) => opts.count(),
            Argument::ConnectOptions(ref opts) => opts.count(),
            Argument::Error(ref vec) => vec.len(),
            Argument::FetchOptions(ref opts) => opts.count(),
            Argument::LobFlags(ref opts) => opts.count(),
            Argument::Parameters(ref pars) => pars.count(),
            Argument::SessionContext(ref opts) => opts.count(),
            Argument::StatementContext(ref sc) => sc.count(),
            Argument::TransactionFlags(ref opts) => opts.count(),
            Argument::XatOptions(ref xat) => xat.count(),
            ref a => {
                return Err(HdbError::Impl(format!("count() called on {:?}", a)));
            }
        })
    }

    // only called on output (serialize)
    pub fn size(&self, with_padding: bool) -> HdbResult<usize> {
        let mut size = 0usize;
        match *self {
            Argument::Auth(ref af) => size += af.size(),
            Argument::ClientContext(ref opts) => size += opts.size(),
            Argument::ClientInfo(ref client_info) => size += client_info.size(),
            Argument::Command(ref s) => size += cesu8::to_cesu8(s).len(),
            Argument::CommandInfo(ref opts) => size += opts.size(),
            Argument::CommitOptions(ref opts) => size += opts.size(),
            Argument::ConnectOptions(ref conn_opts) => size += conn_opts.size(),
            Argument::FetchOptions(ref opts) => size += opts.size(),
            Argument::FetchSize(_) => size += 4,
            Argument::LobFlags(ref opts) => size += opts.size(),
            Argument::Parameters(ref pars) => size += pars.size()?,
            Argument::ReadLobRequest(ref r) => size += r.size(),
            Argument::ResultSetId(_) => size += 8,
            Argument::StatementId(_) => size += 8,
            Argument::StatementContext(ref sc) => size += sc.size(),

            Argument::SessionContext(ref opts) => size += opts.size(),
            Argument::TopologyInformation(ref topology) => size += topology.size(),
            Argument::TransactionFlags(ref taflags) => size += taflags.size(),
            Argument::XatOptions(ref xat) => size += xat.size(),

            ref arg => {
                return Err(HdbError::Impl(format!("size() called on {:?}", arg)));
            }
        }
        if with_padding {
            size += padsize(size);
        }
        trace!("Part_buffer_size = {}", size);
        Ok(size)
    }

    /// Serialize to byte stream
    pub fn serialize(&self, remaining_bufsize: u32, w: &mut io::Write) -> HdbResult<u32> {
        match *self {
            Argument::Auth(ref af) => af.serialize(w)?,
            Argument::ClientContext(ref opts) => opts.serialize(w)?,
            Argument::ClientInfo(ref client_info) => {
                client_info.serialize(w)?;
            }
            Argument::Command(ref s) => {
                for b in cesu8::to_cesu8(s).iter() {
                    w.write_u8(*b)?;
                }
            }
            Argument::CommandInfo(ref opts) => opts.serialize(w)?,
            Argument::CommitOptions(ref opts) => opts.serialize(w)?,
            Argument::ConnectOptions(ref conn_opts) => conn_opts.serialize(w)?,

            Argument::FetchSize(fs) => {
                w.write_u32::<LittleEndian>(fs)?;
            }
            Argument::FetchOptions(ref opts) => opts.serialize(w)?,
            Argument::LobFlags(ref opts) => opts.serialize(w)?,
            Argument::Parameters(ref parameters) => {
                parameters.serialize(w)?;
            }
            Argument::ReadLobRequest(ref r) => r.serialize(w)?,
            Argument::ResultSetId(rs_id) => {
                w.write_u64::<LittleEndian>(rs_id)?;
            }
            Argument::SessionContext(ref opts) => opts.serialize(w)?,
            Argument::StatementId(stmt_id) => {
                w.write_u64::<LittleEndian>(stmt_id)?;
            }
            Argument::StatementContext(ref sc) => sc.serialize(w)?,
            Argument::TransactionFlags(ref taflags) => taflags.serialize(w)?,
            Argument::XatOptions(ref xatid) => xatid.serialize(w)?,
            ref a => {
                return Err(HdbError::Impl(format!("serialize() called on {:?}", a)));
            }
        }

        let size = self.size(false)?;
        let padsize = padsize(size);
        for _ in 0..padsize {
            w.write_u8(0)?;
        }

        trace!(
            "remaining_bufsize: {}, size: {}, padsize: {}",
            remaining_bufsize,
            size,
            padsize
        );
        Ok(remaining_bufsize - size as u32 - padsize as u32)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn parse(
        kind: PartKind,
        attributes: PartAttributes,
        no_of_args: i32,
        parts: &mut Parts,
        o_am_conn_core: Option<&AmConnCore>,
        o_rs_md: Option<&ResultSetMetadata>,
        o_par_md: Option<&Vec<ParameterDescriptor>>,
        o_rs: &mut Option<&mut ResultSet>,
        rdr: &mut io::BufRead,
    ) -> HdbResult<Argument<'a>> {
        trace!("Entering parse(no_of_args={}, kind={:?})", no_of_args, kind);

        let arg = match kind {
            PartKind::Authentication => Argument::Auth(AuthFields::parse(rdr)?),
            PartKind::CommandInfo => Argument::CommandInfo(CommandInfo::parse(no_of_args, rdr)?),
            PartKind::ConnectOptions => {
                Argument::ConnectOptions(ConnectOptions::parse(no_of_args, rdr)?)
            }
            PartKind::Error => Argument::Error(ServerError::parse(no_of_args, rdr)?),
            PartKind::OutputParameters => {
                if let Some(par_md) = o_par_md {
                    Argument::OutputParameters(OutputParameters::parse(
                        o_am_conn_core,
                        par_md,
                        rdr,
                    )?)
                } else {
                    return Err(HdbError::Impl(
                        "Parsing output parameters needs metadata".to_owned(),
                    ));
                }
            }
            PartKind::ParameterMetadata => {
                Argument::ParameterMetadata(ParameterDescriptor::parse(no_of_args, rdr)?)
            }
            PartKind::ReadLobReply => Argument::ReadLobReply(ReadLobReply::parse(rdr)?),
            PartKind::ResultSet => {
                let rs = ResultSet::parse(
                    no_of_args,
                    attributes,
                    parts,
                    o_am_conn_core
                        .ok_or_else(|| HdbError::impl_("ResultSet parsing requires a conn_core"))?,
                    o_rs_md,
                    o_rs,
                    rdr,
                )?;
                Argument::ResultSet(rs)
            }
            PartKind::ResultSetId => Argument::ResultSetId(rdr.read_u64::<LittleEndian>()?),
            PartKind::ResultSetMetadata => {
                Argument::ResultSetMetadata(ResultSetMetadata::parse(no_of_args, rdr)?)
            }
            PartKind::ExecutionResult => {
                Argument::ExecutionResult(ExecutionResult::parse(no_of_args, rdr)?)
            }
            PartKind::StatementContext => {
                Argument::StatementContext(StatementContext::parse(no_of_args, rdr)?)
            }
            PartKind::StatementId => Argument::StatementId(rdr.read_u64::<LittleEndian>()?),
            PartKind::SessionContext => {
                Argument::SessionContext(SessionContext::parse(no_of_args, rdr)?)
            }
            PartKind::TableLocation => {
                let mut vec = Vec::<i32>::new();
                for _ in 0..no_of_args {
                    vec.push(rdr.read_i32::<LittleEndian>()?);
                }
                Argument::TableLocation(vec)
            }
            PartKind::TopologyInformation => {
                Argument::TopologyInformation(Topology::parse(no_of_args, rdr)?)
            }
            PartKind::PartitionInformation => {
                Argument::PartitionInformation(PartitionInformation::parse(rdr)?)
            }
            PartKind::TransactionFlags => {
                Argument::TransactionFlags(TransactionFlags::parse(no_of_args, rdr)?)
            }
            PartKind::XatOptions => Argument::XatOptions(XatOptions::parse(no_of_args, rdr)?),
            _ => {
                return Err(HdbError::Impl(format!(
                    "No handling implemented for received partkind value {}",
                    kind.to_i8()
                )));
            }
        };

        Ok(arg)
    }
}

fn padsize(size: usize) -> usize {
    match size {
        0 => 0,
        _ => 7 - (size - 1) % 8,
    }
}
