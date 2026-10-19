use census_domain::model::{ProfileId, ProfileSelectionV1};
use census_store::{ProfileRegistration, Store};
use std::io::Write;
use std::path::Path;

use super::args::{Operation, ProfileArgs, ProfileCommand, SelectionRequest};
use super::error::{ProfileCliError, ProfileCliResult};
use super::{input, output};

pub(super) fn execute<W: Write>(
    root: Option<&Path>,
    args: &ProfileArgs,
    writer: &mut W,
) -> ProfileCliResult<()> {
    let root = root.ok_or(ProfileCliError::StoreRequired)?;
    match &args.command {
        ProfileCommand::Register(args) => {
            let profile = input::read_profile(&args.file)?;
            let (store, _writer) = Store::open_public_identity_qualification_v3(root)?.into_parts();
            register(&store, &profile, writer)
        }
        ProfileCommand::Show(args) => {
            let (store, _writer) = Store::open_public_identity_qualification_v3(root)?.into_parts();
            show(&store, &args.id, writer)
        }
        ProfileCommand::Select(args) => {
            let request = args.prepare()?;
            let (store, _writer) = Store::open_public_identity_qualification_v3(root)?.into_parts();
            select(&store, request, writer)
        }
    }
}

fn register<W: Write>(
    store: &Store,
    profile: &census_domain::model::QueryProfileV1,
    writer: &mut W,
) -> ProfileCliResult<()> {
    let (status, profile_id) = match store.register_profile(profile)? {
        ProfileRegistration::Registered(id) => (output::Status::Registered, id),
        ProfileRegistration::Repeated(id) => (output::Status::Repeated, id),
    };
    output::reply(
        writer,
        Operation::Register,
        status,
        output::ProfileRef {
            profile_id: &profile_id,
        },
    )
}

fn show<W: Write>(store: &Store, id: &ProfileId, writer: &mut W) -> ProfileCliResult<()> {
    let profile = store.profile(id)?;
    output::reply(
        writer,
        Operation::Show,
        output::Status::Shown,
        output::Shown {
            profile_id: id,
            payload: &profile,
        },
    )
}

fn select<W: Write>(
    store: &Store,
    request: SelectionRequest,
    writer: &mut W,
) -> ProfileCliResult<()> {
    let snapshot = store.snapshot();
    let selection = match request {
        SelectionRequest::LocalId(id) => ProfileSelectionV1::new(snapshot.store_identity()?, id),
        SelectionRequest::Document(selection) => selection,
    };
    snapshot.selected_profile(&selection)?;
    output::reply(
        writer,
        Operation::Select,
        output::Status::SelectionPrepared,
        output::SelectionPrepared::new(&selection),
    )
}
