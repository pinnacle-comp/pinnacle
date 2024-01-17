pub mod pinnacle {

    pub mod v0alpha1 {
        tonic::include_proto!("pinnacle.v0alpha1");
    }

    pub mod input {
        pub mod v0alpha1 {
            tonic::include_proto!("pinnacle.input.v0alpha1");
        }
    }

    pub mod output {
        pub mod v0alpha1 {
            tonic::include_proto!("pinnacle.output.v0alpha1");
        }
    }

    pub mod tag {
        pub mod v0alpha1 {
            tonic::include_proto!("pinnacle.tag.v0alpha1");
        }
    }

    pub mod window {
        pub mod v0alpha1 {
            tonic::include_proto!("pinnacle.window.v0alpha1");
        }
    }

    pub mod process {
        pub mod v0alpha1 {
            tonic::include_proto!("pinnacle.process.v0alpha1");
        }
    }
}

pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("pinnacle");
