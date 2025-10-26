pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("snowcap");

pub mod snowcap {
    pub mod v0alpha1 {
        tonic::include_proto!("snowcap.v0alpha1");
    }

    pub mod v1 {
        tonic::include_proto!("snowcap.v1");
    }

    pub mod widget {
        pub mod v0alpha1 {
            tonic::include_proto!("snowcap.widget.v0alpha1");
        }

        pub mod v1 {
            tonic::include_proto!("snowcap.widget.v1");
        }
    }

    pub mod layer {
        pub mod v0alpha1 {
            tonic::include_proto!("snowcap.layer.v0alpha1");
        }

        pub mod v1 {
            tonic::include_proto!("snowcap.layer.v1");
        }
    }

    pub mod decoration {
        pub mod v1 {
            tonic::include_proto!("snowcap.decoration.v1");
        }
    }

    pub mod operation {
        pub mod v1 {
            tonic::include_proto!("snowcap.operation.v1");
        }
    }

    pub mod input {
        pub mod v0alpha1 {
            tonic::include_proto!("snowcap.input.v0alpha1");
        }

        pub mod v1 {
            tonic::include_proto!("snowcap.input.v1");
        }
    }
}
