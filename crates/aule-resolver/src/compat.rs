use crate::error::ResolveError;
use crate::types::InstallPlan;

/// Check that the install plan has an enabled adapter for the requested runtime target.
///
/// If no specific target was requested, this always succeeds.
pub fn check_adapter_compatibility(
    plan: &InstallPlan,
    requested_target: Option<&str>,
) -> Result<(), ResolveError> {
    let Some(target) = requested_target else {
        return Ok(());
    };

    let has_compatible = plan
        .adapters
        .iter()
        .any(|a| a.runtime_id == target && a.enabled);

    if has_compatible {
        Ok(())
    } else {
        Err(ResolveError::NoCompatibleAdapter {
            name: plan.skill_name.clone(),
            target: target.to_string(),
        })
    }
}
