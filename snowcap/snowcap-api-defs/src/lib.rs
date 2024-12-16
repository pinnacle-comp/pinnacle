pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("snowcap");

pub mod snowcap {
    pub mod v0alpha1 {
        tonic::include_proto!("snowcap.v0alpha1");
    }

    pub mod widget {
        pub mod v0alpha1 {
            tonic::include_proto!("snowcap.widget.v0alpha1");
        }
    }

    pub mod layer {
        pub mod v0alpha1 {
            tonic::include_proto!("snowcap.layer.v0alpha1");
        }
    }

    pub mod input {
        pub mod v0alpha1 {
            tonic::include_proto!("snowcap.input.v0alpha1");
        }
    }
}
