use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use jsonschema::{Draft, JSONSchema};
use minicode_core::prompt::{McpServerSummary, SkillSummary};
use minicode_permissions::PermissionManager;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct ToolContext {
    pub cwd: String,
    pub permissions: Option<Arc<PermissionManager>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BackgroundTaskResult {
    pub task_id: String,
    pub r#type: String,
    pub command: String,
    pub pid: i32,
    pub status: String,
    pub started_at: i64,
}

#[derive(Debug, Clone)]
pub struct ToolResult {
    pub ok: bool,
    pub output: String,
    pub background_task: Option<BackgroundTaskResult>,
    pub await_user: bool,
}

impl ToolResult {
    pub fn ok(output: impl Into<String>) -> Self {
        Self {
            ok: true,
            output: output.into(),
            background_task: None,
            await_user: false,
        }
    }

    pub fn err(output: impl Into<String>) -> Self {
        Self {
            ok: false,
            output: output.into(),
            background_task: None,
            await_user: false,
        }
    }
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> Value;
    async fn run(&self, input: Value, context: &ToolContext) -> ToolResult;
}

enum InputValidator {
    Compiled(JSONSchema),
    CompileError(String),
}

fn compile_validator(schema: &Value) -> InputValidator {
    match JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(schema)
    {
        Ok(validator) => InputValidator::Compiled(validator),
        Err(err) => InputValidator::CompileError(format!("Invalid tool schema: {err}")),
    }
}

fn validate_tool_input(validator: &InputValidator, input: &Value) -> Result<(), String> {
    match validator {
        InputValidator::Compiled(validator) => {
            if let Err(errors) = validator.validate(input) {
                let details = errors.take(3).map(|e| e.to_string()).collect::<Vec<_>>();
                if details.is_empty() {
                    return Err("Invalid input".to_string());
                }
                return Err(format!("Invalid input: {}", details.join("; ")));
            }
            Ok(())
        }
        InputValidator::CompileError(err) => Err(err.clone()),
    }
}

pub struct ToolRegistry {
    state: RwLock<ToolRegistryState>,
}

struct ToolRegistryState {
    tools: Vec<Arc<dyn Tool>>,
    index: HashMap<String, usize>,
    validators: Vec<InputValidator>,
    skills: Vec<SkillSummary>,
    mcp_servers: Vec<McpServerSummary>,
    disposer: Option<Arc<dyn Fn() -> futures::future::BoxFuture<'static, ()> + Send + Sync>>,
}

fn combine_disposers(
    left: Option<Arc<dyn Fn() -> futures::future::BoxFuture<'static, ()> + Send + Sync>>,
    right: Option<Arc<dyn Fn() -> futures::future::BoxFuture<'static, ()> + Send + Sync>>,
) -> Option<Arc<dyn Fn() -> futures::future::BoxFuture<'static, ()> + Send + Sync>> {
    match (left, right) {
        (None, None) => None,
        (Some(one), None) | (None, Some(one)) => Some(one),
        (Some(a), Some(b)) => Some(Arc::new(move || {
            let a = a.clone();
            let b = b.clone();
            Box::pin(async move {
                a().await;
                b().await;
            })
        })),
    }
}

impl ToolRegistry {
    pub fn new(
        tools: Vec<Arc<dyn Tool>>,
        skills: Vec<SkillSummary>,
        mcp_servers: Vec<McpServerSummary>,
        disposer: Option<Arc<dyn Fn() -> futures::future::BoxFuture<'static, ()> + Send + Sync>>,
    ) -> Self {
        let mut index = HashMap::new();
        let mut validators = Vec::with_capacity(tools.len());
        for (idx, tool) in tools.iter().enumerate() {
            index.insert(tool.name().to_string(), idx);
            validators.push(compile_validator(&tool.input_schema()));
        }
        Self {
            state: RwLock::new(ToolRegistryState {
                tools,
                index,
                validators,
                skills,
                mcp_servers,
                disposer,
            }),
        }
    }

    pub fn list(&self) -> Vec<Arc<dyn Tool>> {
        self.state
            .read()
            .ok()
            .map(|state| state.tools.clone())
            .unwrap_or_default()
    }

    pub fn get_skills(&self) -> Vec<SkillSummary> {
        self.state
            .read()
            .ok()
            .map(|state| state.skills.clone())
            .unwrap_or_default()
    }

    pub fn get_mcp_servers(&self) -> Vec<McpServerSummary> {
        self.state
            .read()
            .ok()
            .map(|state| state.mcp_servers.clone())
            .unwrap_or_default()
    }

    pub fn set_mcp_servers(&self, mcp_servers: Vec<McpServerSummary>) {
        if let Ok(mut state) = self.state.write() {
            state.mcp_servers = mcp_servers;
        }
    }

    pub fn extend_dynamic_tools(
        &self,
        tools: Vec<Arc<dyn Tool>>,
        mcp_servers: Vec<McpServerSummary>,
        disposer: Option<Arc<dyn Fn() -> futures::future::BoxFuture<'static, ()> + Send + Sync>>,
    ) {
        if let Ok(mut state) = self.state.write() {
            for tool in tools {
                let name = tool.name().to_string();
                if state.index.contains_key(&name) {
                    continue;
                }
                let idx = state.tools.len();
                state.index.insert(name, idx);
                state
                    .validators
                    .push(compile_validator(&tool.input_schema()));
                state.tools.push(tool);
            }
            state.mcp_servers = mcp_servers;
            state.disposer = combine_disposers(state.disposer.clone(), disposer);
        }
    }

    pub async fn execute(
        &self,
        tool_name: &str,
        input: Value,
        context: &ToolContext,
    ) -> ToolResult {
        let (tool, validation_error) = if let Ok(state) = self.state.read() {
            let Some(idx) = state.index.get(tool_name) else {
                return ToolResult::err(format!("Unknown tool: {tool_name}"));
            };
            let tool = state.tools[*idx].clone();
            let validation_error = validate_tool_input(&state.validators[*idx], &input).err();
            (tool, validation_error)
        } else {
            return ToolResult::err("Tool registry lock poisoned");
        };

        if let Some(err) = validation_error {
            return ToolResult::err(err);
        }

        tool.run(input, context).await
    }

    pub async fn dispose(&self) {
        let disposer = self
            .state
            .read()
            .ok()
            .and_then(|state| state.disposer.clone());
        if let Some(disposer) = disposer {
            disposer().await;
        }
    }
}
