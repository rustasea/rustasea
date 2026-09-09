//! Built-in console command registration.
//!
//! Registration order is stable so `list` output is deterministic: the
//! `make:*` family first, then the framework inspection and operation
//! commands (`route:list`, `show:model`, `queue:*`, `schedule:*`, `migrate`).

pub mod builtins;
pub mod inspect;
pub mod ops;

use crate::registry::CommandRegistry;

/// Register every built-in command into `reg`.
pub fn register_all_into(reg: &mut CommandRegistry) {
    reg.register(builtins::List);
    reg.register(builtins::MakeController);
    reg.register(builtins::MakeModel);
    reg.register(builtins::MakeProvider);
    reg.register(builtins::MakeCommand);
    reg.register(builtins::MakeJob);
    reg.register(builtins::MakeEvent);
    reg.register(builtins::MakeListener);
    reg.register(builtins::MakeObserver);
    reg.register(builtins::MakeTest);
    reg.register(builtins::MakeSeeder);
    reg.register(builtins::MakeMigration);
    reg.register(builtins::MakeAgent);
    reg.register(builtins::MakeTool);
    reg.register(inspect::RouteList);
    reg.register(inspect::ShowModel);
    reg.register(ops::QueueFailed);
    reg.register(ops::QueueRetry);
    reg.register(ops::ScheduleList);
    reg.register(ops::SchedulePause);
    reg.register(ops::ScheduleResume);
    reg.register(ops::ScheduleRun);
    reg.register(ops::Migrate);
}

/// Register every built-in command into the global process registry.
pub fn register_all() {
    let registry = crate::registry();
    let Ok(mut reg) = registry.write() else {
        panic!("global command registry lock poisoned");
    };
    register_all_into(&mut reg);
}
