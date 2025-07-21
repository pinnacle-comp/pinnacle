# snowcap-decoration-v1

- interface snowcap_decoration_manager_v1
    - request get_decoration_surface
        - id: new_id snowcap_decoration_surface_v1
        - surface: object wl_surface,
        - toplevel: object zwlr_foreign_toplevel_handle_v1
    - request destroy
    - enum error

- interface snowcap_decoration_surface_v1
    - request set_z_index
        - index: int
    - request set_geometry
        - x: int
        - y: int
        - width: uint
        - height: uint
    - request ack_configure
        - serial: uint
    - event configure
        - serial: uint
        - width: uint
        - height: uint
