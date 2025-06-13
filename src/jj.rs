use std::{collections::HashMap, rc::Rc, sync::Arc};

use crate::cli::Args;

use anyhow::Result;
use chrono::TimeZone;
use jj_cli::revset_util::RevsetExpressionEvaluator;
use jj_lib::{
    config::StackedConfig,
    id_prefix::IdPrefixContext,
    ref_name::WorkspaceNameBuf,
    repo::{ReadonlyRepo, StoreFactories},
    repo_path::RepoPathUiConverter,
    revset::{
        self, RevsetAliasesMap, RevsetDiagnostics, RevsetExtensions, RevsetIteratorExt,
        RevsetParseContext, RevsetWorkspaceContext, UserRevsetExpression,
    },
    settings::UserSettings,
    store::Store,
    workspace::{Workspace, default_working_copy_factories},
};

pub struct Jj {
    workspace: Workspace,
    repo: Arc<ReadonlyRepo>,
    store: Arc<Store>,
    id_prefix_context: IdPrefixContext,
    user_settings: UserSettings,
    path_converter: RepoPathUiConverter,
    workspace_name: WorkspaceNameBuf,
    revset_aliases_map: RevsetAliasesMap,
    revset_extensions: Arc<RevsetExtensions>,
}

impl Jj {
    pub fn load(args: &Args) -> Result<Self> {
        let user_settings = UserSettings::from_config(StackedConfig::with_defaults())?;
        let store_factories = StoreFactories::default();
        let working_copy_factories = default_working_copy_factories();

        let workspace = Workspace::load(
            &user_settings,
            &args.path,
            &store_factories,
            &working_copy_factories,
        )?;

        let workspace_name = workspace.workspace_name().to_owned();
        let repo = workspace.repo_loader().load_at_head()?;
        let store = workspace.repo_loader().store().clone();
        let revset_aliases_map = RevsetAliasesMap::new();
        let revset_extensions = Arc::new(RevsetExtensions::default());
        let id_prefix_context = IdPrefixContext::new(revset_extensions.clone());
        let path_converter = RepoPathUiConverter::Fs {
            cwd: args.path.to_owned(),
            base: args.path.to_owned(),
        };

        Ok(Jj {
            workspace,
            repo,
            store,
            id_prefix_context,
            user_settings,
            path_converter,
            workspace_name,
            revset_aliases_map,
            revset_extensions,
        })
    }

    pub fn get_commits(&self, revision_str: &str) -> Result<()> {
        let revset_expression = self.parse_revset(revision_str)?;
        let revset = revset_expression.evaluate()?;
        let iter = revset.iter();
        for commit_or_error in iter.commits(&self.store) {
            let commit = commit_or_error?;
            println!("{:?}", commit);
        }

        Ok(())
    }

    fn parse_revset(&self, revision_str: &str) -> Result<RevsetExpressionEvaluator> {
        let mut diagnostics = RevsetDiagnostics::new();
        let context = self.revset_parse_context();
        let (expression, modifier) =
            revset::parse_with_modifier(&mut diagnostics, revision_str, &context)?;
        let (expression, _modifier) = (self.attach_revset_evaluator(expression), modifier);
        Ok(expression)
    }

    fn revset_parse_context(&self) -> RevsetParseContext {
        let workspace_context = RevsetWorkspaceContext {
            path_converter: &self.path_converter,
            workspace_name: &self.workspace_name,
        };
        let now = if let Some(timestamp) = self.user_settings.commit_timestamp() {
            chrono::Local
                .timestamp_millis_opt(timestamp.timestamp.0)
                .unwrap()
        } else {
            chrono::Local::now()
        };
        RevsetParseContext {
            aliases_map: &self.revset_aliases_map,
            local_variables: HashMap::new(),
            user_email: self.user_settings.user_email(),
            date_pattern_context: now.into(),
            extensions: &self.revset_extensions,
            workspace: Some(workspace_context),
        }
    }

    pub fn attach_revset_evaluator(
        &self,
        expression: Rc<UserRevsetExpression>,
    ) -> RevsetExpressionEvaluator<'_> {
        RevsetExpressionEvaluator::new(
            self.repo.as_ref(),
            self.revset_extensions.clone(),
            &self.id_prefix_context,
            expression,
        )
    }
}
