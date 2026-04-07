#[cfg(test)]
mod tests {
    use g_p_source::discovery::{
        is_pid_alive, kill_all_instances, list_instances, read_instance, remove_instance,
        write_instance, InstanceStatus,
    };
    use std::path::Path;
    use tempfile::TempDir;

    /// Override HOME so discovery files go to a temp directory instead of ~/.gps/
    fn with_temp_home<F: FnOnce(&Path)>(f: F) {
        let tmp = TempDir::new().unwrap();
        // We can't easily override dirs::home_dir at runtime, so test the
        // public functions that take explicit workspace paths. The instance
        // files will go to the real ~/.gps/instances/ but we use unique
        // workspace paths that won't collide.
        let fake_workspace = tmp.path().join("test-workspace");
        std::fs::create_dir_all(&fake_workspace).unwrap();
        f(&fake_workspace);
    }

    #[test]
    fn write_and_read_instance() {
        with_temp_home(|workspace| {
            write_instance(workspace, 12345, InstanceStatus::Ready).unwrap();

            let info = read_instance(workspace).unwrap();
            assert!(info.is_some());
            let info = info.unwrap();
            assert_eq!(info.port, 12345);
            assert_eq!(info.status, InstanceStatus::Ready);
            assert_eq!(info.workspace, workspace.to_string_lossy().to_string());

            // Cleanup
            remove_instance(workspace);
            let info = read_instance(workspace).unwrap();
            assert!(info.is_none());
        });
    }

    #[test]
    fn list_instances_finds_current() {
        with_temp_home(|workspace| {
            write_instance(workspace, 54321, InstanceStatus::Indexing).unwrap();

            let instances = list_instances();
            // Our instance should be in the list (PID is current process, so alive)
            let found = instances
                .iter()
                .any(|i| i.workspace == workspace.to_string_lossy().to_string());
            assert!(found, "expected to find our instance in list_instances()");

            // Cleanup
            remove_instance(workspace);
        });
    }

    #[test]
    fn is_pid_alive_current_process() {
        assert!(is_pid_alive(std::process::id()));
    }

    #[test]
    fn is_pid_alive_dead_pid() {
        // PID 99999999 is extremely unlikely to be alive
        assert!(!is_pid_alive(99_999_999));
    }

    #[test]
    fn list_instances_cleans_stale() {
        with_temp_home(|workspace| {
            // Write an instance with a dead PID
            write_instance(workspace, 0, InstanceStatus::Ready).unwrap();

            // Manually patch the PID to a dead value
            let hash = g_p_source::discovery::workspace_hash(workspace);
            let path = dirs::home_dir()
                .unwrap()
                .join(".gps/instances")
                .join(format!("{}.json", hash));
            let contents = std::fs::read_to_string(&path).unwrap();
            let patched = contents.replace(
                &format!("\"pid\": {}", std::process::id()),
                "\"pid\": 99999999",
            );
            std::fs::write(&path, patched).unwrap();

            // list_instances should clean it up
            let instances = list_instances();
            let found = instances
                .iter()
                .any(|i| i.workspace == workspace.to_string_lossy().to_string());
            assert!(!found, "stale instance should have been cleaned up");

            // The file should be gone
            assert!(!path.exists(), "stale discovery file should be deleted");
        });
    }

    #[test]
    fn kill_all_does_not_kill_self() {
        with_temp_home(|workspace| {
            // Write an instance with our own PID
            write_instance(workspace, 0, InstanceStatus::Ready).unwrap();

            // kill_all should skip our own PID
            let killed = kill_all_instances();
            // We shouldn't kill ourselves
            assert_eq!(killed, 0);

            // Cleanup (file is removed by kill_all)
            remove_instance(workspace);
        });
    }
}
