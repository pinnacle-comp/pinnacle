/// A custom implementation of [`smithay::render_elements`] that is not generic but rather
/// implements over the three used renderers.
///
/// This is needed to allow GlesTextures to be easily rendered on winit and udev.
///
/// Also idea from Niri. Ya know this whole compositor is slowly inching towards
/// being a Niri clone lol
#[macro_export]
macro_rules! pinnacle_render_elements {
    (
        $(#[$attr:meta])*
        $vis:vis enum $name:ident {
            $( $(#[$variant_attr:meta])*  $variant:ident = $type:ty),+ $(,)?
        }
    ) => {
        $(#[$attr])*
        $vis enum $name {
            $( $(#[$variant_attr])*  $variant($type)),+
        }

        $(impl From<$type> for $name {
            fn from(x: $type) -> Self {
                Self::$variant(x)
            }
        })+

        $crate::pinnacle_render_elements! {
            @impl $name ($name) () => { $($variant = $type),+ }
        }
    };

    (
        $(#[$attr:meta])*
        $vis:vis enum $name:ident<$generic_name:ident> {
            $( $(#[$variant_attr:meta])* $variant:ident = $type:ty),+ $(,)?
        }
    ) => {
        $(#[$attr])*
        $vis enum $name<$generic_name>
        where
            $generic_name: ::smithay::backend::renderer::Renderer + ::smithay::backend::renderer::ImportAll + ::smithay::backend::renderer::ImportMem,
            $generic_name::TextureId: 'static,
        {
            $( $(#[$variant_attr])* $variant($type)),+
        }

        $(impl<$generic_name> From<$type> for $name<$generic_name>
        where
            $generic_name: ::smithay::backend::renderer::Renderer + ::smithay::backend::renderer::ImportAll + ::smithay::backend::renderer::ImportMem,
            $generic_name::TextureId: 'static,
        {
            fn from(x: $type) -> Self {
                Self::$variant(x)
            }
        })+

        $crate::pinnacle_render_elements! {
            @impl $name () ($name<$generic_name>) => { $($variant = $type),+ }
        }
    };

    (@impl $name:ident ($($name_no_generic:ident)?) ($($name_generic:ident<$generic:ident>)?) => {
        $($variant:ident = $type:ty),+
    }) => {
        impl$(<$generic>)? ::smithay::backend::renderer::element::Element for $name$(<$generic>)?
        $(where
            $generic: ::smithay::backend::renderer::Renderer + ::smithay::backend::renderer::ImportAll + ::smithay::backend::renderer::ImportMem,
            $generic::TextureId: 'static,)?
        {
            fn id(&self) -> &::smithay::backend::renderer::element::Id {
                match self {
                    $($name::$variant(elem) => elem.id()),+
                }
            }

            fn current_commit(&self) -> ::smithay::backend::renderer::utils::CommitCounter {
                match self {
                    $($name::$variant(elem) => elem.current_commit()),+
                }
            }

            fn geometry(
                &self,
                scale: ::smithay::utils::Scale<f64>
            ) -> ::smithay::utils::Rectangle<i32, smithay::utils::Physical> {
                match self {
                    $($name::$variant(elem) => elem.geometry(scale)),+
                }
            }

            fn transform(&self) -> ::smithay::utils::Transform {
                match self {
                    $($name::$variant(elem) => elem.transform()),+
                }
            }

            fn src(&self) -> ::smithay::utils::Rectangle<f64, ::smithay::utils::Buffer> {
                match self {
                    $($name::$variant(elem) => elem.src()),+
                }
            }

            fn damage_since(
                &self,
                scale: ::smithay::utils::Scale<f64>,
                commit: ::std::option::Option<::smithay::backend::renderer::utils::CommitCounter>,
            ) -> ::smithay::backend::renderer::utils::DamageSet<i32, ::smithay::utils::Physical> {
                match self {
                    $($name::$variant(elem) => elem.damage_since(scale, commit)),+
                }
            }

            fn opaque_regions(
                &self,
                scale: ::smithay::utils::Scale<f64>,
            ) -> ::smithay::backend::renderer::utils::OpaqueRegions<i32, ::smithay::utils::Physical> {
                match self {
                    $($name::$variant(elem) => elem.opaque_regions(scale)),+
                }
            }

            fn alpha(&self) -> f32 {
                match self {
                    $($name::$variant(elem) => elem.alpha()),+
                }
            }

            fn kind(&self) -> ::smithay::backend::renderer::element::Kind {
                match self {
                    $($name::$variant(elem) => elem.kind()),+
                }
            }
        }

        impl ::smithay::backend::renderer::element::RenderElement<::smithay::backend::renderer::gles::GlesRenderer>
            for $($name_generic<::smithay::backend::renderer::gles::GlesRenderer>)? $($name_no_generic)?
        {
            fn draw(
                &self,
                frame: &mut ::smithay::backend::renderer::gles::GlesFrame<'_>,
                src: ::smithay::utils::Rectangle<f64, ::smithay::utils::Buffer>,
                dst: ::smithay::utils::Rectangle<i32, ::smithay::utils::Physical>,
                damage: &[::smithay::utils::Rectangle<i32, ::smithay::utils::Physical>],
                opaque_regions: &[::smithay::utils::Rectangle<i32, ::smithay::utils::Physical>],
            ) -> ::std::result::Result<(), ::smithay::backend::renderer::gles::GlesError> {
                match self {
                    $($name::$variant(elem) => {
                        ::smithay::backend::renderer::element::RenderElement::<
                            ::smithay::backend::renderer::gles::GlesRenderer
                        >::draw(elem, frame, src, dst, damage, opaque_regions)
                    })+
                }
            }

            fn underlying_storage(
                &self,
                renderer: &mut ::smithay::backend::renderer::gles::GlesRenderer
            ) -> ::std::option::Option<::smithay::backend::renderer::element::UnderlyingStorage> {
                match self {
                    $($name::$variant(elem) => elem.underlying_storage(renderer)),+
                }
            }
        }

        impl<'a> ::smithay::backend::renderer::element::RenderElement<$crate::backend::udev::UdevRenderer<'a>>
            for $($name_generic<$crate::backend::udev::UdevRenderer<'a>>)? $($name_no_generic)?
        {
            fn draw(
                &self,
                frame: &mut <$crate::backend::udev::UdevRenderer<'a> as ::smithay::backend::renderer::Renderer>::Frame<'_>,
                src: ::smithay::utils::Rectangle<f64, ::smithay::utils::Buffer>,
                dst: ::smithay::utils::Rectangle<i32, ::smithay::utils::Physical>,
                damage: &[::smithay::utils::Rectangle<i32, ::smithay::utils::Physical>],
                opaque_regions: &[::smithay::utils::Rectangle<i32, ::smithay::utils::Physical>],
            ) -> ::std::result::Result<
                (),
                <$crate::backend::udev::UdevRenderer as ::smithay::backend::renderer::Renderer>::Error,
            > {
                match self {
                    $($name::$variant(elem) => {
                        ::smithay::backend::renderer::element::RenderElement::<
                            $crate::backend::udev::UdevRenderer
                        >::draw(elem, frame, src, dst, damage, opaque_regions)
                    })+
                }
            }

            fn underlying_storage(
                &self,
                renderer: &mut $crate::backend::udev::UdevRenderer<'a>,
            ) -> ::std::option::Option<::smithay::backend::renderer::element::UnderlyingStorage> {
                match self {
                    $($name::$variant(elem) => elem.underlying_storage(renderer)),+
                }
            }
        }

        #[cfg(feature = "testing")]
        impl ::smithay::backend::renderer::element::RenderElement<::smithay::backend::renderer::test::DummyRenderer>
            for $($name_generic<::smithay::backend::renderer::test::DummyRenderer>)? $($name_no_generic)?
        {
            fn draw(
                &self,
                frame: &mut <::smithay::backend::renderer::test::DummyRenderer as ::smithay::backend::renderer::Renderer>::Frame<'_>,
                src: ::smithay::utils::Rectangle<f64, ::smithay::utils::Buffer>,
                dst: ::smithay::utils::Rectangle<i32, ::smithay::utils::Physical>,
                damage: &[::smithay::utils::Rectangle<i32, ::smithay::utils::Physical>],
                opaque_regions: &[::smithay::utils::Rectangle<i32, ::smithay::utils::Physical>],
            ) -> ::std::result::Result<
                (),
                <::smithay::backend::renderer::test::DummyRenderer as ::smithay::backend::renderer::Renderer>::Error,
            > {
                match self {
                    $($name::$variant(elem) => {
                        ::smithay::backend::renderer::element::RenderElement::<
                            ::smithay::backend::renderer::test::DummyRenderer
                        >::draw(elem, frame, src, dst, damage, opaque_regions)
                    })+
                }
            }

            fn underlying_storage(
                &self,
                renderer: &mut ::smithay::backend::renderer::test::DummyRenderer,
            ) -> ::std::option::Option<::smithay::backend::renderer::element::UnderlyingStorage> {
                match self {
                    $($name::$variant(elem) => elem.underlying_storage(renderer)),+
                }
            }
        }
    }
}
