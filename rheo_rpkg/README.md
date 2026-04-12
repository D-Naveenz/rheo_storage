# rheo_rpkg

`rheo_rpkg` is the generic `RPKG` v2 container crate for the Rheo workspace.

It stores MessagePack payloads with optional MessagePack metadata, optional
integrity sections, and package purposes that tell readers whether to prefer a
fast payload path or a fuller metadata/integrity path by default.
