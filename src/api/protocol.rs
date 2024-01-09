pub mod pinnacle {
    pub mod v0alpha1 {
        tonic::include_proto!("pinnacle.v0alpha1");
    }

    pub mod input {
        pub mod v0alpha1 {
            tonic::include_proto!("pinnacle.input.v0alpha1");
        }

        pub mod libinput {
            pub mod v0alpha1 {
                tonic::include_proto!("pinnacle.input.libinput.v0alpha1");
            }
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

        pub mod rules {
            pub mod v0alpha1 {
                tonic::include_proto!("pinnacle.window.rules.v0alpha1");
            }
        }
    }
}
