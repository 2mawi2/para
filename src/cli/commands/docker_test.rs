#[cfg(test)]
mod tests {
    use crate::cli::parser::{StartArgs, DispatchArgs};

    #[test]
    fn test_start_args_docker_image() {
        // Test that StartArgs accepts docker_image
        let args = StartArgs {
            name: Some("test".to_string()),
            dangerously_skip_permissions: false,
            container: true,
            allow_domains: None,
            docker_args: vec![],
            docker_image: Some("custom:latest".to_string()),
            no_forward_keys: false,
        };
        
        assert_eq!(args.docker_image, Some("custom:latest".to_string()));
        assert!(!args.no_forward_keys);
    }

    #[test]
    fn test_dispatch_args_docker_image() {
        // Test that DispatchArgs accepts docker_image
        let args = DispatchArgs {
            name_or_prompt: Some("test prompt".to_string()),
            prompt: None,
            file: None,
            dangerously_skip_permissions: false,
            container: true,
            allow_domains: None,
            docker_args: vec![],
            docker_image: Some("python:3.11".to_string()),
            no_forward_keys: false,
        };
        
        assert_eq!(args.docker_image, Some("python:3.11".to_string()));
        assert!(!args.no_forward_keys);
    }
    
    #[test]
    fn test_no_forward_keys_flag() {
        // Test the no_forward_keys flag
        let args = StartArgs {
            name: Some("secure".to_string()),
            dangerously_skip_permissions: false,
            container: true,
            allow_domains: None,
            docker_args: vec![],
            docker_image: Some("untrusted:latest".to_string()),
            no_forward_keys: true,
        };
        
        assert!(args.no_forward_keys);
        
        let dispatch_args = DispatchArgs {
            name_or_prompt: Some("secure task".to_string()),
            prompt: None,
            file: None,
            dangerously_skip_permissions: false,
            container: true,
            allow_domains: None,
            docker_args: vec![],
            docker_image: Some("public:latest".to_string()),
            no_forward_keys: true,
        };
        
        assert!(dispatch_args.no_forward_keys);
    }
}