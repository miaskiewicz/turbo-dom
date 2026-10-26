# Consolidation test move manifest (S1)

Every test of turbo-html2pdf-core @ e2d7c4d and turbo-surf-raster @ b1de4eb, old id ->
new id, from the per-test results of `cargo test` (`--no-fail-fast`, rustc 1.98.0) in each
baseline configuration before the move and after it (`consol/s1-layout` in turbo-dom and
turbo-html2pdf). An id is `binary<source>::test`; `<src/lib.rs>` are in-source unit tests.

Configurations (columns `ws`/`all`/`nodef`/`svg`): html2pdf `cargo test --workspace` vs
turbo-dom-layout `--features xref,pdf-ua,print-color` (the features the html2pdf workspace
unifies onto layout); core `--all-features` vs layout `--all-features`; core
`--no-default-features` vs layout `--no-default-features`; core `--features svg` vs layout
`--features svg`. Cell = result in that configuration (identical before and after; `-` = not
compiled in that configuration). raster: turbo-surf `--workspace` vs `cargo test -p
turbo-dom-raster`.

Categories: **verbatim** — the test moved unchanged (whole file or in-source module moved;
only the crate path rewritten); **split** — the test moved out of a mixed file whose TKind /
templating / PDF cases stay in turbo-html2pdf-core; **stays** — still in
turbo-html2pdf-core (PDF/templating/TKind); **stays (split file)** — the core half of a
split file. No test is copied (the `tests/common/mod.rs` font helper is the only shared
file present in both crates).

Totals: split: 184, stays: 254, stays (split file): 106, verbatim: 342; raster verbatim: 64.
Every old id exists exactly once after the move with the same result in every configuration (0 mismatches).

Regenerate: run the four configurations with `scratchpad/scripts/baseline-cargo.sh`, then
`s1/manifest.py` (consolidation scratchpad).

## turbo-html2pdf-core -> turbo-dom-layout / turbo-html2pdf-core

| old id | new crate | new id | category | ws | all | nodef | svg |
|---|---|---|---|---|---|---|---|
| `append<tests/append.rs>::append_is_byte_deterministic` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `append<tests/append.rs>::append_with_no_extras_returns_base_pages` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `append<tests/append.rs>::appends_multiple_extras_in_order` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `append<tests/append.rs>::appends_single_extra_sums_pages` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `append<tests/append.rs>::malformed_base_is_error` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `append<tests/append.rs>::malformed_extra_is_error` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `append<tests/append.rs>::merged_output_passes_qpdf_check` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `append<tests/append.rs>::no_pages_is_error` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `bundled_fonts<tests/bundled_fonts.rs>::bundled_set_loads_all_faces` | turbo-dom-layout | (same) | split | ok | ok | - | ok |
| `bundled_fonts<tests/bundled_fonts.rs>::caller_face_overrides_bundled_for_same_family` | turbo-dom-layout | (same) | split | ok | ok | - | ok |
| `bundled_fonts<tests/bundled_fonts.rs>::generic_families_map_to_bundled_primaries` | turbo-dom-layout | (same) | split | ok | ok | - | ok |
| `bundled_fonts<tests/bundled_fonts.rs>::pdf_embeds_bundled_monospace_and_serif` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | - | ok |
| `bundled_fonts<tests/bundled_fonts.rs>::pdf_embeds_bundled_sans_face` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | - | ok |
| `bundled_fonts<tests/bundled_fonts.rs>::renders_with_no_caller_fonts_and_no_css` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | - | ok |
| `complex_layout_harness<tests/complex_layout_harness.rs>::absolute_siblings_no_offset_share_cb_origin` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `complex_layout_harness<tests/complex_layout_harness.rs>::auto_inset_absolute_uses_static_position_not_cb_origin` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `complex_layout_harness<tests/complex_layout_harness.rs>::class_rule_from_style_block_applies` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `complex_layout_harness<tests/complex_layout_harness.rs>::multi_class_element_gets_all_matching_rules` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `complex_layout_harness<tests/complex_layout_harness.rs>::sr_only_clip_path_inset_is_not_painted` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `complex_layout_harness<tests/complex_layout_harness.rs>::sr_only_clip_rect_is_not_painted` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `complex_layout_harness<tests/complex_layout_harness.rs>::sr_only_via_class_not_painted` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `complex_layout_harness<tests/complex_layout_harness.rs>::tiny_overflow_hidden_box_clips_its_text` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `conformance_flags<tests/conformance_flags.rs>::all_flags_off_emits_no_conformance_machinery` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `conformance_flags<tests/conformance_flags.rs>::all_flags_on_coexist_with_non_colliding_ids` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `conformance_flags<tests/conformance_flags.rs>::cmyk_toggle_switches_colour_space` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `conformance_flags<tests/conformance_flags.rs>::qpdf_accepts_combined_flags` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `emit<tests/emit.rs>::bundled_cff_and_truetype_text_is_extractable_without_pdf_ua` | turbo-html2pdf-core | (same) | stays | ok | ok | - | ok |
| `emit<tests/emit.rs>::default_metadata_uses_sentinel_date` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `emit<tests/emit.rs>::directive_fragments_and_empty_bands_are_skipped` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `emit<tests/emit.rs>::embeds_truetype_and_cff_fonts_with_children` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `emit<tests/emit.rs>::emit_is_byte_deterministic` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `emit<tests/emit.rs>::emits_valid_pdf_for_every_fixture` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `emit<tests/emit.rs>::font_store_reports_emptiness` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `emit<tests/emit.rs>::full_metadata_round_trips` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `emit<tests/emit.rs>::geometry_uses_edges_margins` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `emit<tests/emit.rs>::header_footer_footnote_bands_paint` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `emit<tests/emit.rs>::ordinary_pdf_fonts_have_unicode_maps_and_extract_original_text` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `emit<tests/emit.rs>::out_of_range_date_falls_back_to_sentinel` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `emit<tests/emit.rs>::paints_boxes_and_borders` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `emit<tests/emit.rs>::qpdf_check_when_available` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `emit<tests/emit.rs>::unsubsettable_glyphs_fall_back_to_full_embed` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `encrypt<tests/encrypt.rs>::key_material_has_the_spec_lengths` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `encrypt<tests/encrypt.rs>::owner_and_user_passwords_round_trip_through_qpdf` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `encrypt<tests/encrypt.rs>::permissions_round_trip_through_p` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `encrypt<tests/encrypt.rs>::plaintext_strings_and_streams_are_encrypted` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `encrypt<tests/encrypt.rs>::user_password_round_trips_through_qpdf` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `encrypt<tests/encrypt.rs>::writes_v5_r6_aesv3_encrypt_dict` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `encryption_roundtrip<tests/encryption_roundtrip.rs>::lopdf_decrypts_turbos_v5_r6_aesv3_output_and_recovers_the_marker` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `encryption_roundtrip<tests/encryption_roundtrip.rs>::lopdf_reencrypts_the_decrypted_document_and_only_the_new_password_opens_it` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `encryption_roundtrip<tests/encryption_roundtrip.rs>::lopdf_rejects_the_wrong_password` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `endnotes<tests/endnotes.rs>::endnote_becomes_numbered_superscript_marker` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `endnotes<tests/endnotes.rs>::multiple_notes_number_in_document_order` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `endnotes<tests/endnotes.rs>::nested_endnote_and_slot_are_found` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `endnotes<tests/endnotes.rs>::no_directives_is_identity` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `endnotes<tests/endnotes.rs>::notes_without_a_slot_collect_but_emit_no_list` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `endnotes<tests/endnotes.rs>::render_flows_note_body_into_the_endnotes_slot` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `endnotes<tests/endnotes.rs>::second_slot_is_dropped` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `filters<tests/filters.rs>::ac_2_24_currency_euro_locale` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `filters<tests/filters.rs>::ac_3_6e_now_pinned_for_determinism` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `filters<tests/filters.rs>::currency_anglo_default_locale` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `filters<tests/filters.rs>::currency_negative_and_symbols` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `filters<tests/filters.rs>::currency_unknown_code_uses_code_prefix` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `filters<tests/filters.rs>::date_bad_string_errors` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `filters<tests/filters.rs>::date_bad_tz_no_colon_errors` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `filters<tests/filters.rs>::date_bad_tz_no_sign_errors` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `filters<tests/filters.rs>::date_bad_tz_non_numeric_errors` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `filters<tests/filters.rs>::date_from_plain_date_string` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `filters<tests/filters.rs>::date_from_rfc3339_with_offset_tz` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `filters<tests/filters.rs>::date_from_timestamp_default_fmt` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `filters<tests/filters.rs>::date_negative_offset_tz` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `filters<tests/filters.rs>::date_non_timestamp_value_errors` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `filters<tests/filters.rs>::date_timestamp_out_of_range_errors` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `filters<tests/filters.rs>::date_tz_out_of_range_errors` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `filters<tests/filters.rs>::datetime_default_fmt_and_utc_tz` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `filters<tests/filters.rs>::now_unset_is_render_error` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `filters<tests/filters.rs>::number_default_and_explicit_decimals` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `filters<tests/filters.rs>::number_negative_keeps_sign` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `filters<tests/filters.rs>::ordinal_suffixes` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `filters<tests/filters.rs>::pad_default_and_custom_fill` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `filters<tests/filters.rs>::percent_default_and_decimals` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `filters<tests/filters.rs>::truncate_variants` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `filters<tests/filters.rs>::wordwrap_wraps_and_handles_empty` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `fixtures<tests/fixtures.rs>::every_fixture_compiles_and_renders` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `fixtures<tests/fixtures.rs>::fixtures_are_deterministic` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `footnotes<tests/footnotes.rs>::ac_3_15_footnote_lands_on_its_reference_page` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `footnotes<tests/footnotes.rs>::ac_3_16_default_numbering_is_document_continuous` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `footnotes<tests/footnotes.rs>::ac_3_16_page_reset_keeps_manual_marks` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `footnotes<tests/footnotes.rs>::ac_3_16_page_reset_restarts_numbering_each_page` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `footnotes<tests/footnotes.rs>::ac_3_17_a_footnote_reduces_body_capacity_on_its_page` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `footnotes<tests/footnotes.rs>::ac_3_18_oversized_note_continues_across_pages` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `footnotes<tests/footnotes.rs>::ac_3_19_separator_rule_heads_the_footnote_band` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `footnotes<tests/footnotes.rs>::ac_3_20_manual_mark_overrides_auto_number` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `footnotes<tests/footnotes.rs>::ac_6_4_converges_without_lint_in_the_common_case` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `footnotes<tests/footnotes.rs>::ac_6_4_non_convergence_is_linted` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `footnotes<tests/footnotes.rs>::footnote_rendering_is_deterministic` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `footnotes<tests/footnotes.rs>::no_footnotes_leaves_bands_empty` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `hackernews_layout_harness<tests/hackernews_layout_harness.rs>::a_link_pseudo_color_overrides_ua_blue` | turbo-dom-layout | (same) | verbatim | ok | ok | FAILED | ok |
| `hackernews_layout_harness<tests/hackernews_layout_harness.rs>::class_descendant_color_overrides_ua_blue` | turbo-dom-layout | (same) | verbatim | ok | ok | FAILED | ok |
| `hackernews_layout_harness<tests/hackernews_layout_harness.rs>::double_br_creates_vertical_gap` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `hackernews_layout_harness<tests/hackernews_layout_harness.rs>::nested_full_width_table_fills_its_cell` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `hackernews_layout_harness<tests/hackernews_layout_harness.rs>::percent_width_table_column_fills_not_double_applied` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `hackernews_layout_harness<tests/hackernews_layout_harness.rs>::sized_img_lays_out_at_top_left_at_its_size` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `hackernews_layout_harness<tests/hackernews_layout_harness.rs>::table_width_percent_stretches_to_container` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `image<tests/image.rs>::alpha_png_emits_smask` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `image<tests/image.rs>::background_image_emits_xobject_behind_box` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `image<tests/image.rs>::build_box_tree_records_img_source` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `image<tests/image.rs>::decode_cmyk_jpeg_is_unsupported` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `image<tests/image.rs>::decode_grayscale_jpeg_is_device_gray_passthrough` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `image<tests/image.rs>::decode_jpeg_passes_through_as_dct` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `image<tests/image.rs>::decode_png_grayscale_alpha_splits_plane` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `image<tests/image.rs>::decode_png_grayscale_keeps_one_component` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `image<tests/image.rs>::decode_png_rgb_to_raw_samples` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `image<tests/image.rs>::decode_png_rgba_splits_alpha_into_smask_plane` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `image<tests/image.rs>::decode_rejects_malformed_input` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `image<tests/image.rs>::degenerate_zero_dimension_falls_back_to_given` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `image<tests/image.rs>::emits_image_xobject_for_resolved_img` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `image<tests/image.rs>::explicit_height_fills_width_from_aspect_ratio` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `image<tests/image.rs>::explicit_width_and_height_are_used_verbatim` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `image<tests/image.rs>::explicit_width_fills_height_from_aspect_ratio` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `image<tests/image.rs>::flex_img_item_with_explicit_width_uses_it` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `image<tests/image.rs>::flex_item_wrapping_img_sizes_to_img_intrinsic` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `image<tests/image.rs>::flex_row_img_item_sizes_to_intrinsic_not_zero` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `image<tests/image.rs>::flex_row_item_wrapping_img_natural_width_uses_intrinsic` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `image<tests/image.rs>::grayscale_png_emits_device_gray` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `image<tests/image.rs>::height_cap_clamps_to_60_percent_of_body_height` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `image<tests/image.rs>::height_cap_inactive_without_body_height` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `image<tests/image.rs>::image_emit_is_deterministic` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `image<tests/image.rs>::image_store_collect_dedups_and_reports_size` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `image<tests/image.rs>::image_store_skips_undecodable_names_during_collect` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `image<tests/image.rs>::image_store_skips_unresolvable_names_during_collect` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `image<tests/image.rs>::intrinsic_size_used_when_it_fits` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `image<tests/image.rs>::jpeg_emits_dct_decode` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `image<tests/image.rs>::layout_html_with_images_sizes_a_resolvable_img` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `image<tests/image.rs>::mask_image_box_emits_tinted_image_and_no_solid_background` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `image<tests/image.rs>::no_image_default_path_skips_image_fragments` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `image<tests/image.rs>::no_images_resolver_resolves_nothing` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `image<tests/image.rs>::percent_height_absolute_img_resolves_against_positioned_ancestor` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `image<tests/image.rs>::percent_height_img_resolves_against_definite_parent` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `image<tests/image.rs>::percent_height_resolves_against_cb_height_when_known` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `image<tests/image.rs>::percent_width_resolves_against_containing_block_not_zero` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `image<tests/image.rs>::probe_reads_dimensions_and_alpha` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `image<tests/image.rs>::qpdf_check_image_pdf_when_available` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `image<tests/image.rs>::sniff_recognizes_png_jpeg_and_rejects_others` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `image<tests/image.rs>::unresolved_image_emits_nothing_and_lays_out_zero_sized` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `image<tests/image.rs>::width_cap_clamps_to_containing_block_and_preserves_aspect` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout<tests/layout.rs>::block_elements_stack` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `layout<tests/layout.rs>::inline_elements_stay_on_one_line` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `layout<tests/layout.rs>::layout_is_deterministic` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `layout<tests/layout.rs>::node_ids_round_trip_to_the_box_tree` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `layout<tests/layout.rs>::paragraph_renders_text` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `layout<tests/layout.rs>::real_table_lays_out_via_ua_defaults` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::absolute_calc_percent_minus_px_top_positions_inside_parent` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::absolute_flex_overlay_fills_its_band_at_card_bottom` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::absolute_percent_height_child_uses_relative_parent_content_height` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::absolute_right_inset_anchors_to_right_edge` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::aspect_ratio_ignored_when_height_is_explicit` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::aspect_ratio_sizes_auto_height_from_width` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::auto_width_fills_and_padding_offsets_content` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::auto_width_inline_block_shrinks_to_content` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::background_and_border_become_box_content` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::block_directive_is_zero_size_marker` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::border_box_min_height_includes_padding_and_border` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::border_box_width_includes_padding` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::border_radius_auto_resolves_to_zero` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::break_properties_propagate_to_meta` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::content_box_width_adds_padding` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::deferred_absolute_with_top_inset_anchors_from_the_top` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::deferred_absolute_without_vertical_inset_keeps_static_y` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::empty_block_collapses_through` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::empty_registry_renders_no_text_lines` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::explicit_height_is_honored` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::flex_and_table_fall_back_to_block_flow` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::floats_drop_below_when_row_full` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::floats_narrow_region_and_clear_drops_below` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::in_flow_content_flows_beside_floats` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::in_flow_percent_height_stays_content_derived` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::inline_block_auto_width_honors_min_width` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::inline_block_flows_within_the_line_next_to_text` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::inline_blocks_flow_horizontally` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::inline_blocks_wrap_when_row_full` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::left_and_right_floats_go_to_opposite_edges` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::left_floats_pack_side_by_side` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::lines_handle_text_directive_and_atomic` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::margin_inline_start_maps_to_left_offset` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::mask_image_without_resolver_skips_tint` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::max_height_clamps_taller_content` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::min_and_max_width_clamp` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::oversized_float_stays_put_no_finite_drop` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::percent_border_radius_resolves_against_box` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::percent_height_positioned_box_resolves_against_definite_ancestor` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::right_float_narrows_line_region_from_the_right` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::right_float_table_reanchors_to_right_edge` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::shrink_float_with_inline_block_measures_content` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::sibling_margins_collapse_to_max` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::tall_float_forces_clear_block_to_drop_below` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::text_align_center_and_margin_auto_center_width_constrained_blocks` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::text_align_right_pushes_block_child_to_right` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_block<tests/layout_block.rs>::visibility_hidden_and_opacity_zero_drop_the_box` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_boxgen<tests/layout_boxgen.rs>::background_shorthand_url_becomes_image_source` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_boxgen<tests/layout_boxgen.rs>::block_directive_is_opaque_marker` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `layout_boxgen<tests/layout_boxgen.rs>::block_nested_in_inline_becomes_atomic` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_boxgen<tests/layout_boxgen.rs>::display_none_is_dropped` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_boxgen<tests/layout_boxgen.rs>::empty_flow_is_empty_lines` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_boxgen<tests/layout_boxgen.rs>::flex_container_keeps_only_real_items` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_boxgen<tests/layout_boxgen.rs>::flex_keeps_directive_child` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `layout_boxgen<tests/layout_boxgen.rs>::img_lazy_data_url_and_srcset_become_replaced_source` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_boxgen<tests/layout_boxgen.rs>::inline_block_becomes_atomic` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_boxgen<tests/layout_boxgen.rs>::inline_directive_is_inline_item` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `layout_boxgen<tests/layout_boxgen.rs>::inline_element_drops_hidden_child` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_boxgen<tests/layout_boxgen.rs>::input_button_value_renders_as_text` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_boxgen<tests/layout_boxgen.rs>::mask_shorthand_url_is_extracted_via_token_scan` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_boxgen<tests/layout_boxgen.rs>::mixed_block_inline_wraps_anonymous_block` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_boxgen<tests/layout_boxgen.rs>::node_ids_are_preorder` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_boxgen<tests/layout_boxgen.rs>::nonblank_inline_run_between_blocks_makes_anonymous_box` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_boxgen<tests/layout_boxgen.rs>::table_structure_preserved` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_boxgen<tests/layout_boxgen.rs>::text_only_flow_is_inline_context` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_boxgen<tests/layout_boxgen.rs>::whitespace_between_blocks_makes_no_anonymous_box` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_flex<tests/layout_flex.rs>::align_keywords_render` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_flex<tests/layout_flex.rs>::auto_basis_keyword_is_accepted` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_flex<tests/layout_flex.rs>::container_min_height_prevents_collapse` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_flex<tests/layout_flex.rs>::content_sized_item_uses_natural_width` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_flex<tests/layout_flex.rs>::deeply_nested_flex_is_not_exponential` | turbo-dom-layout | (same) | split | ok | ok | FAILED | ok |
| `layout_flex<tests/layout_flex.rs>::direction_keywords_render` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_flex<tests/layout_flex.rs>::directive_item_has_zero_natural_width` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `layout_flex<tests/layout_flex.rs>::empty_flex_container_has_no_items` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_flex<tests/layout_flex.rs>::explicit_basis_sizes_items` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_flex<tests/layout_flex.rs>::explicit_width_item_sizes_to_width` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_flex<tests/layout_flex.rs>::gap_adds_space_between_items` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_flex<tests/layout_flex.rs>::grow_splits_free_space_equally` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_flex<tests/layout_flex.rs>::item_min_height_applies` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_flex<tests/layout_flex.rs>::justify_keywords_render` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_flex<tests/layout_flex.rs>::nested_block_item_measures_children` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_flex<tests/layout_flex.rs>::percent_width_item_fills_via_flex_basis` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_flex<tests/layout_flex.rs>::row_flex_fits_margin_spaced_items` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_flex<tests/layout_flex.rs>::wrap_keywords_render_and_wrap` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_fragment<tests/layout_fragment.rs>::break_meta_defaults` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_fragment<tests/layout_fragment.rs>::directive_content_roundtrips` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `layout_fragment<tests/layout_fragment.rs>::fragment_new_defaults_and_bottom` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_fragment<tests/layout_fragment.rs>::node_id_default_is_zero` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_fragment<tests/layout_fragment.rs>::repeatable_marks` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_fragment<tests/layout_fragment.rs>::text_line_holds_glyphs_and_face` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_fragment<tests/layout_fragment.rs>::translate_moves_subtree` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_grid<tests/layout_grid.rs>::column_gap_spaces_tracks` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_grid<tests/layout_grid.rs>::empty_grid_container_has_no_items` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_grid<tests/layout_grid.rs>::fixed_px_and_fr_columns_mix` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_grid<tests/layout_grid.rs>::grid_column_and_row_span_widen_items` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_grid<tests/layout_grid.rs>::minmax_track_fills_remaining_width` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_grid<tests/layout_grid.rs>::named_areas_place_items_by_grid_area` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_grid<tests/layout_grid.rs>::repeat_expands_track_list` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_grid<tests/layout_grid.rs>::third_item_wraps_to_second_row` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_grid<tests/layout_grid.rs>::two_fr_columns_split_width_side_by_side` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_inline<tests/layout_inline.rs>::adjacent_identical_runs_merge` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_inline<tests/layout_inline.rs>::alignment_shifts_line` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_inline<tests/layout_inline.rs>::empty_and_whitespace_produce_no_lines` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_inline<tests/layout_inline.rs>::explicit_line_height_sets_box` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_inline<tests/layout_inline.rs>::leading_and_trailing_whitespace_collapse` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_inline<tests/layout_inline.rs>::letter_spacing_widens` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_inline<tests/layout_inline.rs>::long_word_takes_its_own_line` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_inline<tests/layout_inline.rs>::missing_glyph_emits_notdef_lint` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_inline<tests/layout_inline.rs>::mixed_font_sizes_grow_line_box` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_inline<tests/layout_inline.rs>::multi_run_word_keeps_distinct_faces` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_inline<tests/layout_inline.rs>::nowrap_run_stays_on_one_line` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_inline<tests/layout_inline.rs>::per_glyph_fallback_finds_another_face` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_inline<tests/layout_inline.rs>::single_run_one_line` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_inline<tests/layout_inline.rs>::superscript_raises_subscript_lowers` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_inline<tests/layout_inline.rs>::vertical_align_places_inline_block_atoms` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_inline<tests/layout_inline.rs>::wraps_to_multiple_lines_and_stacks` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_position<tests/layout_position.rs>::absolute_is_relative_to_positioned_ancestor` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_position<tests/layout_position.rs>::absolute_is_removed_from_flow` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_position<tests/layout_position.rs>::absolute_lands_at_its_insets` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_position<tests/layout_position.rs>::fixed_is_relative_to_the_page_origin` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_position<tests/layout_position.rs>::lower_z_index_paints_under_higher` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_position<tests/layout_position.rs>::negative_z_paints_under_in_flow` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_position<tests/layout_position.rs>::positioned_paints_over_in_flow_sibling` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_position<tests/layout_position.rs>::relative_shifts_box_but_keeps_its_space` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_snapshot<tests/layout_snapshot.rs>::all_text_lines_keep_source_and_flattened_inline_ancestry_without_fake_range_rects` | turbo-dom-layout | (same) | split | ok | ok | FAILED | ok |
| `layout_snapshot<tests/layout_snapshot.rs>::ancestor_and_child_transforms_compose_about_actual_box_origins` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_snapshot<tests/layout_snapshot.rs>::anonymous_contents_text_matches_independent_chrome155_geometry` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_snapshot<tests/layout_snapshot.rs>::anonymous_items_keep_original_text_identity_and_unboxed_ancestry` | turbo-dom-layout | (same) | split | ok | ok | FAILED | ok |
| `layout_snapshot<tests/layout_snapshot.rs>::anonymous_wrappers_never_receive_the_id_of_a_neighboring_source` | turbo-dom-layout | (same) | split | ok | ok | FAILED | ok |
| `layout_snapshot<tests/layout_snapshot.rs>::authored_live_boxes_match_independently_recorded_chrome155_geometry` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_snapshot<tests/layout_snapshot.rs>::contents_ancestors_keep_source_identity_without_frame_or_transform` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_snapshot<tests/layout_snapshot.rs>::contents_boxes_match_independently_recorded_chrome155_geometry` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_snapshot<tests/layout_snapshot.rs>::contents_promotes_children_to_real_flex_items` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_snapshot<tests/layout_snapshot.rs>::contents_text_keeps_its_own_inherited_line_height` | turbo-dom-layout | (same) | split | ok | ok | FAILED | ok |
| `layout_snapshot<tests/layout_snapshot.rs>::directive_fragments_keep_actual_source_identity_and_category` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `layout_snapshot<tests/layout_snapshot.rs>::double_precision_used_transforms_follow_real_image_fragments_once_and_keep_legacy_matrix_type` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_snapshot<tests/layout_snapshot.rs>::generated_input_label_has_no_fabricated_text_source_but_retains_owner` | turbo-dom-layout | (same) | split | ok | ok | FAILED | ok |
| `layout_snapshot<tests/layout_snapshot.rs>::hidden_and_clipped_boxes_keep_geometry_but_display_none_has_no_fragments` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_snapshot<tests/layout_snapshot.rs>::hidden_directives_and_their_descendants_have_no_live_geometry` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `layout_snapshot<tests/layout_snapshot.rs>::invalid_quad_declarations_preserve_prior_valid_sides_and_inherited_variables_expand` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_snapshot<tests/layout_snapshot.rs>::legacy_float_translation_is_retained_for_nonperspective_z_translation` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_snapshot<tests/layout_snapshot.rs>::live_anonymous_inline_boxes_do_not_repeat_parent_frame_or_transform` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_snapshot<tests/layout_snapshot.rs>::live_document_root_contains_body_margins_and_preserves_legacy_pdf_policy` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_snapshot<tests/layout_snapshot.rs>::partial_source_maps_do_not_invent_missing_identities_and_empty_tree_is_valid` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_snapshot<tests/layout_snapshot.rs>::positioned_shorthands_keep_cascade_order_and_importance` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_snapshot<tests/layout_snapshot.rs>::replaced_boxes_keep_overflow_css_frames_transforms_and_all_image_fragments` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_snapshot<tests/layout_snapshot.rs>::replaced_constraints_honor_box_sizing_ratio_and_min_over_max` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_snapshot<tests/layout_snapshot.rs>::replaced_contents_suppresses_fallback_descendants_and_br_breaks` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_snapshot<tests/layout_snapshot.rs>::source_identity_survives_moves_without_guest_attributes_or_tree_rewrite` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_snapshot<tests/layout_snapshot.rs>::source_validation_rejects_duplicates_outside_paths_and_bad_viewports` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_snapshot<tests/layout_snapshot.rs>::transformed_fixed_admission_is_identity_independent_and_shared_by_both_entry_points` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_snapshot<tests/layout_snapshot.rs>::used_box_metrics_are_captured_before_transforms_and_follow_final_translations` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_snapshot<tests/layout_snapshot.rs>::viewport_and_definite_ancestor_heights_resolve_real_percentage_boxes` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_snapshot<tests/layout_snapshot.rs>::zero_fragment_provenance_distinguishes_excluded_subtrees_from_empty_visible_inlines` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `layout_table<tests/layout_table.rs>::auto_layout_never_below_min_content` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_table<tests/layout_table.rs>::auto_layout_scales_down_on_overflow` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_table<tests/layout_table.rs>::auto_layout_shrinks_to_content` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_table<tests/layout_table.rs>::border_spacing_separates_cells_by_default` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_table<tests/layout_table.rs>::collapse_tall_rowspan_grows_last_spanned_row` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_table<tests/layout_table.rs>::colspan_cell_can_exceed_column_sum` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_table<tests/layout_table.rs>::colspan_widens_cell` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_table<tests/layout_table.rs>::empty_spacer_row_keeps_its_explicit_height` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_table<tests/layout_table.rs>::empty_table_has_no_rows` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_table<tests/layout_table.rs>::explicit_table_width_scales_columns` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_table<tests/layout_table.rs>::fixed_collapse_zero_width_columns_scale_to_zero` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_table<tests/layout_table.rs>::fixed_layout_all_columns_explicit` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_table<tests/layout_table.rs>::fixed_layout_all_px_columns_leave_no_auto_share` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_table<tests/layout_table.rs>::fixed_layout_auto_column_has_no_explicit_width` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_table<tests/layout_table.rs>::fixed_layout_honors_explicit_then_shares` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_table<tests/layout_table.rs>::fixed_layout_splits_equally` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_table<tests/layout_table.rs>::plain_row_group_is_not_repeatable` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_table<tests/layout_table.rs>::rowspan_cell_taller_than_rows_expands` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_table<tests/layout_table.rs>::rowspan_occupies_next_row_column` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_table<tests/layout_table.rs>::simple_two_by_two_grid` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_table<tests/layout_table.rs>::span_attrs_tolerate_garbage` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_table<tests/layout_table.rs>::text_only_group_contributes_no_cells` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_table<tests/layout_table.rs>::thead_tfoot_rows_are_repeatable` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_table<tests/layout_table.rs>::vertical_align_shifts_cell_content` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_value<tests/layout_value.rs>::align_white_space_valign` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_value<tests/layout_value.rs>::all_sides_border_color_longhand_overrides_shorthand` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_value<tests/layout_value.rs>::background_shorthand_resolves_color` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_value<tests/layout_value.rs>::border_keyword_widths` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_value<tests/layout_value.rs>::borders_shorthand_side_and_longhand` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_value<tests/layout_value.rs>::box_sizing_and_position` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_value<tests/layout_value.rs>::breaks_orphans_widows_background` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_value<tests/layout_value.rs>::color_hex_forms` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_value<tests/layout_value.rs>::color_named_and_invalid` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_value<tests/layout_value.rs>::color_resolution` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_value<tests/layout_value.rs>::color_rgb_forms` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_value<tests/layout_value.rs>::display_maps_all` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_value<tests/layout_value.rs>::edges_helpers` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_value<tests/layout_value.rs>::font_families_split` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_value<tests/layout_value.rs>::font_size_resolution` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_value<tests/layout_value.rs>::font_weight_and_style` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_value<tests/layout_value.rs>::from_pairs_get_roundtrip` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_value<tests/layout_value.rs>::length_pct_resolves` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_value<tests/layout_value.rs>::letter_spacing_resolves` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_value<tests/layout_value.rs>::line_height_forms` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_value<tests/layout_value.rs>::margin_longhand_overrides_shorthand` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_value<tests/layout_value.rs>::margin_shorthand_arity` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_value<tests/layout_value.rs>::padding_percent_against_cb_width` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_value<tests/layout_value.rs>::parse_calc_additive_lengths` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_value<tests/layout_value.rs>::parse_length_pct_variants` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_value<tests/layout_value.rs>::parse_px_rejects_pct_and_garbage` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_value<tests/layout_value.rs>::parse_px_units` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_value<tests/layout_value.rs>::parse_viewport_units_resolve_against_layout_viewport` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_value<tests/layout_value.rs>::rem_is_root_relative_not_parent_relative` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `layout_value<tests/layout_value.rs>::width_height_min_max` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `native_block_inline<tests/native_block_inline.rs>::all_static_split_variants_preserve_original_source_ownership` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `native_block_inline<tests/native_block_inline.rs>::block_in_inline_fragments_and_neighbor_geometry_match_chrome` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `native_block_inline<tests/native_block_inline.rs>::escaping_percentage_bottom_margin_uses_its_actual_containing_width` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `native_block_inline<tests/native_block_inline.rs>::fragment_ownership_preserves_ids_and_internal_href_does_not_atomize_native_blocks` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `native_block_inline<tests/native_block_inline.rs>::hidden_blocks_do_not_displace_or_reject_a_relative_inline` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `native_block_inline<tests/native_block_inline.rs>::partial_source_identities_keep_the_real_block_without_inventing_inline_owners` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `native_block_inline<tests/native_block_inline.rs>::real_empty_blocks_keep_their_height_borders_and_flow_advance` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `native_block_inline<tests/native_block_inline.rs>::saved_chrome_probe_provenance_matches_reusable_source` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `native_block_inline<tests/native_block_inline.rs>::unsupported_displaced_splits_reject_but_boxless_text_does_not` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `native_font_run_advances<tests/native_font_run_advances.rs>::native_rotated_element_and_partial_text_use_their_distinct_chrome_bounds` | turbo-dom-layout | (same) | verbatim | ok | ok | - | ok |
| `native_font_run_advances<tests/native_font_run_advances.rs>::native_whole_source_runs_spaces_partial_ranges_and_neighbors_match_normal_chrome` | turbo-dom-layout | (same) | verbatim | ok | ok | - | ok |
| `native_font_run_advances<tests/native_font_run_advances.rs>::ordinary_pdf_paragraph_and_generic_snapshot_keep_unquantized_real_advances` | turbo-dom-layout | (same) | verbatim | ok | ok | - | ok |
| `native_font_run_advances<tests/native_font_run_advances.rs>::pinned_real_fonts_and_normal_oracle_provenance_are_unchanged` | turbo-dom-layout | (same) | verbatim | ok | ok | - | ok |
| `native_iframe_sizing<tests/native_iframe_sizing.rs>::native_iframe_default_object_size_attributes_css_and_zero_boxes_match_chrome` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `native_iframe_sizing<tests/native_iframe_sizing.rs>::native_iframe_fallback_sources_remain_identifiable_without_rendered_glyphs` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `native_iframe_sizing<tests/native_iframe_sizing.rs>::native_zero_viewport_retains_fixed_descendant_and_viewport_unit_geometry` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `native_inline_borders<tests/native_inline_borders.rs>::authored_inline_graphs_retain_actual_source_roles_on_every_platform` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `native_inline_borders<tests/native_inline_borders.rs>::authored_native_inline_fragments_match_independent_chrome_boundaries_fonts_and_slices` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `native_inline_borders<tests/native_inline_borders.rs>::ordinary_inline_and_replaced_source_fragments_remain_distinct` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `native_inline_borders<tests/native_inline_borders.rs>::relative_inline_offsets_move_descendants_without_changing_following_flow` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `native_inline_borders<tests/native_inline_borders.rs>::transparent_ancestors_reference_child_regions_without_repainting_child_alpha_or_borders` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `native_inline_borders<tests/native_inline_borders.rs>::unsupported_ordinary_inline_contexts_reject_without_inventing_partial_boxes` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `native_inline_coverage<tests/native_inline_coverage.rs>::adjacent_unbroken_source_runs_and_multiline_inline_metrics_retain_their_owners` | turbo-dom-layout | (same) | split | ok | ok | FAILED | ok |
| `native_inline_coverage<tests/native_inline_coverage.rs>::an_unmatched_public_closing_edge_keeps_its_advance_without_inventing_glyphs` | turbo-dom-layout | (same) | split | ok | ok | FAILED | ok |
| `native_inline_coverage<tests/native_inline_coverage.rs>::collapsed_empty_inline_sources_keep_zero_boxes_and_relative_position` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `native_inline_coverage<tests/native_inline_coverage.rs>::empty_inline_inside_visible_text_keeps_its_insertion_box_without_advance` | turbo-dom-layout | (same) | split | ok | ok | FAILED | ok |
| `native_inline_coverage<tests/native_inline_coverage.rs>::hidden_subtrees_and_contents_wrappers_preserve_visible_source_geometry` | turbo-dom-layout | (same) | split | ok | ok | FAILED | ok |
| `native_inline_coverage<tests/native_inline_coverage.rs>::native_fixed_replacements_resolve_percentage_height_against_the_viewport` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `native_inline_coverage<tests/native_inline_coverage.rs>::own_atomic_super_and_sub_alignment_shift_once_without_growing_a_dominant_strut` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `native_inline_coverage<tests/native_inline_coverage.rs>::preserved_whitespace_and_inline_directives_keep_real_source_provenance` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | FAILED | ok |
| `native_inline_coverage<tests/native_inline_coverage.rs>::public_source_paragraphs_keep_edge_advances_and_optional_font_metrics` | turbo-dom-layout | (same) | split | ok | ok | FAILED | ok |
| `native_inline_coverage<tests/native_inline_coverage.rs>::zero_advance_cells_do_not_add_empty_rectangles_to_a_nonempty_selection` | turbo-dom-layout | (same) | split | ok | ok | FAILED | ok |
| `native_inline_spacing_coverage<tests/native_inline_spacing_coverage.rs>::generic_pdf_whitespace_ranges_keep_raw_placed_gaps_without_native_source_cells` | turbo-dom-layout | (same) | verbatim | ok | ok | - | ok |
| `native_inline_spacing_coverage<tests/native_inline_spacing_coverage.rs>::generic_snapshot_and_pdf_atom_caption_keep_raw_font_advances` | turbo-dom-layout | (same) | verbatim | ok | ok | - | ok |
| `native_inline_spacing_coverage<tests/native_inline_spacing_coverage.rs>::leading_collapsible_space_after_images_and_inline_blocks_keeps_native_sources` | turbo-dom-layout | (same) | verbatim | ok | ok | - | ok |
| `native_inline_spacing_coverage<tests/native_inline_spacing_coverage.rs>::ordinary_wrapping_keeps_trailing_whitespace_with_its_previous_word` | turbo-dom-layout | (same) | verbatim | ok | ok | - | ok |
| `native_inline_spacing_coverage<tests/native_inline_spacing_coverage.rs>::rendered_collapsible_space_keeps_its_real_caret_edges_and_provenance` | turbo-dom-layout | (same) | verbatim | ok | ok | - | ok |
| `native_range_transform_stages<tests/native_range_transform_stages.rs>::native_range_projection_matches_margin_padding_absolute_and_nested_chrome_stages` | turbo-dom-layout | (same) | verbatim | ok | ok | - | ok |
| `native_range_transform_stages<tests/native_range_transform_stages.rs>::normal_chrome_transform_oracle_pins_actual_source_font_and_all_cases` | turbo-dom-layout | (same) | verbatim | ok | ok | - | ok |
| `native_serif_policy<tests/native_serif_policy.rs>::native_serif_and_explicit_physical_font_ranges_match_fresh_chrome` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `native_serif_policy<tests/native_serif_policy.rs>::native_serif_primary_is_physical_times_while_pdf_keeps_its_existing_alias` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `nodes<tests/nodes.rs>::ac_2_6_safe_output_becomes_real_markup_node` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `nodes<tests/nodes.rs>::all_t_directive_names_map` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `nodes<tests/nodes.rs>::data_driven_t_element` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `nodes<tests/nodes.rs>::html_comments_are_dropped` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `nodes<tests/nodes.rs>::html_element_attrs_and_children` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `nodes<tests/nodes.rs>::node_accessors_cover_both_arms` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `nodes<tests/nodes.rs>::self_closing_t_elements_are_siblings_not_nested` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `nodes<tests/nodes.rs>::self_closing_with_attrs_expands` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `nodes<tests/nodes.rs>::t_element_becomes_typed_directive` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `nodes<tests/nodes.rs>::truncated_t_tag_without_close_does_not_panic` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `nodes<tests/nodes.rs>::unknown_t_element_errors` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `original_text_units<tests/original_text_units.rs>::original_script_fallback_ranges_match_chrome_without_changing_replacement_projection` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `original_text_units<tests/original_text_units.rs>::public_original_units_retain_native_source_identity_and_validation` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `original_text_units<tests/original_text_units.rs>::real_registered_fallback_tables_keep_source_line_ranges_on_hosts_without_coretext` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `paginate<tests/paginate.rs>::ac_6_0_page_count_is_data_driven` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `paginate<tests/paginate.rs>::ac_6_1_content_width_excludes_margins` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `paginate<tests/paginate.rs>::ac_6_1_default_is_a4_20mm` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `paginate<tests/paginate.rs>::ac_6_1_explicit_wh_and_margin_two_values` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `paginate<tests/paginate.rs>::ac_6_1_invalid_margin_is_ignored` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `paginate<tests/paginate.rs>::ac_6_1_keyword_only_size_falls_back_to_a4` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `paginate<tests/paginate.rs>::ac_6_1_landscape_swaps` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `paginate<tests/paginate.rs>::ac_6_1_margin_four_values` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `paginate<tests/paginate.rs>::ac_6_1_named_letter` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `paginate<tests/paginate.rs>::ac_6_1_portrait_keyword` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `paginate<tests/paginate.rs>::ac_6_1_single_dim_is_square` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `paginate<tests/paginate.rs>::ac_6_1_unknown_named_size_is_error` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `paginate<tests/paginate.rs>::ac_6_1_unknown_property_and_blank_decls_ignored` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `paginate<tests/paginate.rs>::ac_6_2_atomic_leaf_overflow_after_content` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `paginate<tests/paginate.rs>::ac_6_2_atomic_leaf_overflow_on_empty_page` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `paginate<tests/paginate.rs>::ac_6_2_body_translated_into_page_coords` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `paginate<tests/paginate.rs>::ac_6_2_break_inside_avoid_moves_whole` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `paginate<tests/paginate.rs>::ac_6_2_empty_document_yields_one_page` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `paginate<tests/paginate.rs>::ac_6_2_forced_break_after` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `paginate<tests/paginate.rs>::ac_6_2_forced_break_before` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `paginate<tests/paginate.rs>::ac_6_2_greedy_fill_splits_pages` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `paginate<tests/paginate.rs>::ac_6_2_orphans_defer_run_to_next_page` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `paginate<tests/paginate.rs>::ac_6_2_orphans_unsatisfiable_on_empty_page_places_anyway` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `paginate<tests/paginate.rs>::ac_6_2_oversized_block_splits_into_children` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `paginate<tests/paginate.rs>::ac_6_2_single_line_taller_than_page_overflows` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `paginate<tests/paginate.rs>::ac_6_2_trailing_empty_page_trimmed` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `paginate<tests/paginate.rs>::ac_6_2_widows_pull_lines_back` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `paginate<tests/paginate.rs>::ac_6_3_repeatable_header_reemitted` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `paginate<tests/paginate.rs>::ac_6_page_kind_first_left_right` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `pdf_a<tests/pdf_a.rs>::emits_output_intent_icc_and_xmp` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `pdf_a<tests/pdf_a.rs>::empty_metadata_omits_optional_xmp_properties` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `pdf_a<tests/pdf_a.rs>::output_is_byte_deterministic` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `pdf_a<tests/pdf_a.rs>::pdf_a_false_emits_no_pdfa_objects_under_pdf_a_build` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `pdf_a<tests/pdf_a.rs>::qpdf_check_clean` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `pdf_a<tests/pdf_a.rs>::verapdf_validates_2b` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `pdf_a<tests/pdf_a.rs>::watermark_fade_transparency_is_suppressed` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `pdf_a<tests/pdf_a.rs>::xmp_mirrors_info_dict` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `pdf_ua<tests/pdf_ua.rs>::box_decoration_is_an_artifact` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `pdf_ua<tests/pdf_ua.rs>::carries_the_pdfua_xmp_identifier` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `pdf_ua<tests/pdf_ua.rs>::emits_the_tagged_pdf_skeleton` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `pdf_ua<tests/pdf_ua.rs>::passes_verapdf_ua1` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `pdf_ua<tests/pdf_ua.rs>::pdf_ua_false_emits_no_tagged_machinery_under_pdf_ua_build` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `pdf_ua<tests/pdf_ua.rs>::qpdf_check_is_clean` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `pdf_ua<tests/pdf_ua.rs>::tags_the_semantic_roles` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `perf<tests/perf.rs>::a_face_parses_the_font_once` | turbo-dom-layout | (same) | split | - | ok | - | - |
| `perf<tests/perf.rs>::invoice_stage_timing` | turbo-html2pdf-core | (same) | stays (split file) | - | ok | - | - |
| `perf<tests/perf.rs>::report_hot_path_breakdown` | turbo-html2pdf-core | (same) | stays (split file) | - | FAILED | - | - |
| `perf<tests/perf.rs>::shaping_runs_once_per_distinct_run` | turbo-dom-layout | (same) | split | - | ok | - | - |
| `perf<tests/perf.rs>::snapshot_exposes_named_counters` | turbo-dom-layout | (same) | split | - | ok | - | - |
| `positioning_harness<tests/positioning_harness.rs>::absolute_anchors_to_relative_block` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `positioning_harness<tests/positioning_harness.rs>::absolute_anchors_to_relative_inside_float` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `positioning_harness<tests/positioning_harness.rs>::absolute_anchors_to_relative_inside_inline_block` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `positioning_harness<tests/positioning_harness.rs>::absolute_anchors_to_relative_table_cell` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `positioning_harness<tests/positioning_harness.rs>::nested_absolute_anchors_to_absolute_ancestor` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `print_color<tests/print_color.rs>::cmyk_components_clamp_out_of_range` | turbo-dom-layout | (same) | split | ok | ok | - | - |
| `print_color<tests/print_color.rs>::cmyk_false_is_byte_identical_device_rgb_under_print_color_build` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | - | - |
| `print_color<tests/print_color.rs>::cmyk_function_parses_percentages_and_fractions` | turbo-dom-layout | (same) | split | ok | ok | - | - |
| `print_color<tests/print_color.rs>::cmyk_function_rejects_malformed_input` | turbo-dom-layout | (same) | split | ok | ok | - | - |
| `print_color<tests/print_color.rs>::fills_are_emitted_as_device_cmyk_not_rgb` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | - | - |
| `print_color<tests/print_color.rs>::pure_black_is_all_key_ink` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | - | - |
| `print_color<tests/print_color.rs>::pure_red_maps_to_magenta_plus_yellow_ink` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | - | - |
| `print_color<tests/print_color.rs>::pure_white_is_no_ink` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | - | - |
| `render<tests/render.rs>::ac_3_0_3_footer_band_extent_reserved_from_content` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `render<tests/render.rs>::ac_3_0_3_header_band_reserved_and_painted` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `render<tests/render.rs>::ac_6_7_footer_page_number_is_late_evaluated_per_page` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `render<tests/render.rs>::ac_6_7_page_one_and_two_footers_differ_in_number` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `render<tests/render.rs>::ac_6_8_region_taller_than_band_is_clipped_and_linted` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `render<tests/render.rs>::footer_can_interpolate_document_data_and_page_state` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `render<tests/render.rs>::no_region_leaves_bands_empty` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `render<tests/render.rs>::page_context_exposes_is_first_and_is_last` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `render<tests/render.rs>::program_reports_which_regions_are_present` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `render<tests/render.rs>::render_pages_is_deterministic` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `render<tests/render.rs>::render_region_returns_none_for_absent_region` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `render<tests/render.rs>::reserving_a_band_lowers_body_capacity` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `smoke_html_fragment<tests/smoke_html_fragment.rs>::raw_html_to_positioned_fragment` | turbo-dom-layout | (same) | verbatim | ok | ok | FAILED | ok |
| `stamp<tests/stamp.rs>::inherited_media_box_from_the_pages_tree_is_used_when_the_page_has_none` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `stamp<tests/stamp.rs>::malformed_input_is_error` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `stamp<tests/stamp.rs>::missing_media_box_everywhere_falls_back_to_a4` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `stamp<tests/stamp.rs>::no_pages_is_error` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `stamp<tests/stamp.rs>::password_given_for_a_plaintext_input_is_a_decrypt_error` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `stamp<tests/stamp.rs>::stamp_accepts_turbos_raw_encrypted_bytes_with_no_test_side_length_patch` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `stamp<tests/stamp.rs>::stamp_is_deterministic` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `stamp<tests/stamp.rs>::stamped_output_passes_qpdf_check` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `stamp<tests/stamp.rs>::stamps_a_watermark_onto_every_page_preserving_content_and_page_count` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `stamp<tests/stamp.rs>::stamps_an_encrypted_pdf_and_only_the_new_password_opens_the_result` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `stamp<tests/stamp.rs>::wrong_password_on_an_encrypted_input_is_a_decrypt_error` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `style<tests/style.rs>::ac_4_2_later_rule_wins_on_equal_specificity` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::ac_4_2_specificity_id_beats_class_beats_type` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::ac_4_3_inheritance` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::ac_4_4_nth_child_zebra_and_first_last` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::ac_4_6_token_beats_sheet_inline_beats_token` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::ac_4_8_node_styles_render_time` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::ac_4_9_full_six_level_order` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::attribute_selector_negative_cases` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::attribute_selectors_all_ops` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::author_css_overrides_presentational_hint` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::bgcolor_attribute_maps_to_background` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::checked_gates_sibling_reveal_like_wikipedia_dropdowns` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::child_vs_descendant_combinator` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::declaration_semicolon_inside_url_is_not_a_separator` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::directive_elements_are_styled_too` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::em_font_size_does_not_compound_on_inheritance` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::empty_pseudo_matches_childless_element` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::empty_selector_list_produces_no_rule` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `style<tests/style.rs>::enabled_and_disabled_pseudos_on_form_controls` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::font_color_attribute_maps_to_color` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::font_size_keyword_resolves_to_px` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::font_size_rem_and_percent_and_unknown_units` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::hidden_input_is_not_rendered` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::hover_never_matches_in_static_render` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::important_inline_beats_important_author` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::malformed_declarations_are_skipped` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `style<tests/style.rs>::media_calc_single_term_breakpoint` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::media_em_breakpoint_uses_16px_root` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::media_height_gates_rules` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::media_min_width_applies_above_breakpoint` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::media_print_is_dropped_for_screen` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::media_query_without_space_after_at_media_applies` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::nested_braces_in_at_rule_stay_intact` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `style<tests/style.rs>::next_sibling_combinator` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::next_sibling_combinator_tight` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::non_inherited_property_does_not_descend` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::not_pseudo_excludes` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::nth_child_odd_and_an_plus_b` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::nth_child_still_parses_with_paren_arg` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::nth_of_type_counts_per_tag` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::of_type_pseudos_count_per_tag` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::only_child_pseudo_matches_lone_child` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::parses_rules_comments_and_at_rules` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `style<tests/style.rs>::pseudo_element_rule_does_not_style_the_element` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::root_pseudo_matches_html_element` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `style<tests/style.rs>::selector_list_comma_inside_attribute_value_is_not_a_separator` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::selector_list_comma_inside_is_is_not_a_list_separator` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::self_referential_var_uses_fallback` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::subsequent_sibling_combinator` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::superscript_ua_is_smaller_and_raised` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::supports_and_applies_when_both_hold` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::supports_nested_group_recurses` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::supports_not_negates` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::supports_or_applies_when_either_holds` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::supports_unmatched_paren_group_is_treated_as_supported` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::table_cellpadding_pads_its_cells` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::text_nodes_have_no_style` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::token_extends_cycle_terminates` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::tokens_extends_and_later_token_wins` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::ua_default_overridden_by_author` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::universal_selector_matches_all` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::unknown_pseudo_class_is_ignored` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::unknown_token_is_ignored` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::unterminated_comment_consumes_rest` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `style<tests/style.rs>::var_multiple_in_one_value` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::var_nested_reference` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::var_resolves_inherited_custom_property` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::var_uses_fallback_when_undefined` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::var_with_nested_function_fallback` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::var_with_unclosed_paren_passes_through` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::where_pseudo_matches_only_its_argument` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::where_with_nested_not_matches` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style<tests/style.rs>::width_height_attributes_map_to_lengths` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `style_matched<tests/style_matched.rs>::attribute_hints_are_explicit_candidates_overridden_by_author_rules` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `style_matched<tests/style_matched.rs>::important_encapsulation_reverses_with_nested_shadow_contexts` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `style_matched<tests/style_matched.rs>::inherited_variables_shorthand_and_font_size_use_existing_resolution` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `style_matched<tests/style_matched.rs>::normal_encapsulation_beats_specificity_inline_and_source_order` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `style_matched<tests/style_matched.rs>::origin_precedes_scope_and_reverses_for_important_rules` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `style_matched<tests/style_matched.rs>::same_scope_inline_specificity_and_order_remain_independent` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `style_matched<tests/style_matched.rs>::ua_defaults_are_available_without_inherited_style_becoming_a_rule` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `svg<tests/svg.rs>::decode_routes_svg_through_svg_path` | turbo-dom-layout | (same) | split | - | ok | - | ok |
| `svg<tests/svg.rs>::decode_svg_is_byte_deterministic` | turbo-dom-layout | (same) | split | - | ok | - | ok |
| `svg<tests/svg.rs>::decode_svg_yields_rgb_samples_and_alpha_plane` | turbo-dom-layout | (same) | split | - | ok | - | ok |
| `svg<tests/svg.rs>::is_svg_recognizes_svg_and_rejects_raster` | turbo-dom-layout | (same) | split | - | ok | - | ok |
| `svg<tests/svg.rs>::malformed_svg_emits_nothing` | turbo-html2pdf-core | (same) | stays (split file) | - | ok | - | ok |
| `svg<tests/svg.rs>::malformed_svg_is_handled_without_panic` | turbo-dom-layout | (same) | split | - | ok | - | ok |
| `svg<tests/svg.rs>::probe_routes_svg_through_svg_path` | turbo-dom-layout | (same) | split | - | ok | - | ok |
| `svg<tests/svg.rs>::probe_svg_reports_oversampled_size_and_alpha` | turbo-dom-layout | (same) | split | - | ok | - | ok |
| `svg<tests/svg.rs>::prolog_svg_decodes` | turbo-dom-layout | (same) | split | - | ok | - | ok |
| `svg<tests/svg.rs>::qpdf_check_svg_pdf_when_available` | turbo-html2pdf-core | (same) | stays (split file) | - | ok | - | ok |
| `svg<tests/svg.rs>::svg_emit_is_deterministic` | turbo-html2pdf-core | (same) | stays (split file) | - | ok | - | ok |
| `svg<tests/svg.rs>::svg_emits_image_xobject_with_smask` | turbo-html2pdf-core | (same) | stays (split file) | - | ok | - | ok |
| `svg<tests/svg.rs>::zero_sized_svg_decodes_to_nothing` | turbo-dom-layout | (same) | split | - | ok | - | ok |
| `svg_viewport_snapshot<tests/svg_viewport_snapshot.rs>::native_outer_svg_viewport_boxes_match_independent_chrome_without_shape_geometry` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `svg_viewport_snapshot<tests/svg_viewport_snapshot.rs>::native_viewport_layout_scale_is_bounded_and_configurable_before_box_allocation` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `svg_viewport_snapshot<tests/svg_viewport_snapshot.rs>::nested_svg_declarations_fail_and_undeclared_nested_sources_remain_explicit` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `svg_viewport_snapshot<tests/svg_viewport_snapshot.rs>::text_paths_directives_and_invalid_viewports_cannot_be_declared_svg_sources` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `svg_viewport_snapshot<tests/svg_viewport_snapshot.rs>::viewport_declaration_errors_do_not_erase_or_reinterpret_source_identities` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `switch<tests/switch.rs>::ac_2_9b_subject_evaluated_once` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `switch<tests/switch.rs>::ac_2_9c_membership_first_match_wins` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `switch<tests/switch.rs>::ac_2_9d_only_first_body_renders` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `switch<tests/switch.rs>::ac_2_9e_default_must_be_last` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `switch<tests/switch.rs>::ac_2_9e_single_default` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `switch<tests/switch.rs>::ac_2_9f_no_text_before_first_case` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `switch<tests/switch.rs>::ac_2_9f_whitespace_before_case_is_ok` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `switch<tests/switch.rs>::ac_2_9g_non_taken_case_has_no_side_effects` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `switch<tests/switch.rs>::ac_2_9h_case_values_can_be_variables` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `switch<tests/switch.rs>::ac_2_9i_whitespace_control_markers` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `switch<tests/switch.rs>::ac_2_9j_switch_equals_hand_written_if` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `switch<tests/switch.rs>::case_value_with_brackets_is_not_split_on_inner_comma` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `switch<tests/switch.rs>::nested_switch` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `switch<tests/switch.rs>::stray_case_and_endswitch_error` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `switch<tests/switch.rs>::unterminated_jinja_tag_surfaces_as_syntax_error` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `switch<tests/switch.rs>::unterminated_switch_errors` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `templating<tests/templating.rs>::ac_2_0_pure_jinja_renders` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `templating<tests/templating.rs>::ac_2_0b_program_renders_many_times` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `templating<tests/templating.rs>::ac_2_10_dict_iteration_is_insertion_order` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `templating<tests/templating.rs>::ac_2_11_loop_object` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `templating<tests/templating.rs>::ac_2_12_for_else_empty` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `templating<tests/templating.rs>::ac_2_13_nested_loops_distinct_state` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `templating<tests/templating.rs>::ac_2_14_include_partial` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `templating<tests/templating.rs>::ac_2_15_macro_local_scope` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `templating<tests/templating.rs>::ac_2_17_include_recursion_depth_capped` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `templating<tests/templating.rs>::ac_2_18_strict_undefined_errors` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `templating<tests/templating.rs>::ac_2_19_empty_policy_renders_blank` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `templating<tests/templating.rs>::ac_2_1_infix_precedence` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `templating<tests/templating.rs>::ac_2_21_default_filter_fallback` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `templating<tests/templating.rs>::ac_2_22_trim_blocks_default_on` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `templating<tests/templating.rs>::ac_2_23_comments_stripped` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `templating<tests/templating.rs>::ac_2_2_path_access_left_to_right` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `templating<tests/templating.rs>::ac_2_3_filters_chainable` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `templating<tests/templating.rs>::ac_2_4_truthiness_matches_jinja` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `templating<tests/templating.rs>::ac_2_5_autoescape_on` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `templating<tests/templating.rs>::ac_2_6_safe_filter_unescaped` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `templating<tests/templating.rs>::ac_2_8_conditionals_one_branch` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `templating<tests/templating.rs>::ac_2_9_membership_in` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `templating<tests/templating.rs>::ac_9_1_bad_partial_is_compile_error` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `templating<tests/templating.rs>::ac_9_1_syntax_error_has_span` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `templating<tests/templating.rs>::unknown_filter_is_typed_error` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `templating<tests/templating.rs>::unknown_function_is_typed_error` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `templating<tests/templating.rs>::unknown_test_is_typed_error` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `text<tests/text.rs>::alignment_offsets` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `text<tests/text.rs>::collapses_whitespace` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `text<tests/text.rs>::debug_shows_family_and_style` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `text<tests/text.rs>::empty_text_yields_no_lines` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `text<tests/text.rs>::glyph_coverage` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `text<tests/text.rs>::glyph_index_reuses_parsed_face` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `text<tests/text.rs>::invalid_font_bytes_return_none` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `text<tests/text.rs>::line_height_override_is_used` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `text<tests/text.rs>::loads_otf_cff_outlines` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `text<tests/text.rs>::loads_ttf_and_reads_metrics` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `text<tests/text.rs>::measure_scales_with_size_and_letter_spacing` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `text<tests/text.rs>::nowrap_never_breaks` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `text<tests/text.rs>::pre_preserves_explicit_line_breaks` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `text<tests/text.rs>::registry_glyph_fallback_chain` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `text<tests/text.rs>::registry_select_by_weight_and_style` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `text<tests/text.rs>::registry_unknown_family_falls_back_to_first` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `text<tests/text.rs>::shape_produces_glyphs` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `text<tests/text.rs>::short_text_is_one_line` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `text<tests/text.rs>::wraps_into_multiple_lines` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `text_source_clusters<tests/text_source_clusters.rs>::actual_snapshot_run_ids_distinguish_identical_wrapped_words_and_keep_source_and_transforms` | turbo-dom-layout | (same) | verbatim | ok | ok | FAILED | ok |
| `text_source_clusters<tests/text_source_clusters.rs>::actual_text_source_keeps_collapsed_whitespace_and_distinct_owner_inputs` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `text_source_clusters<tests/text_source_clusters.rs>::adjacent_input_runs_keep_distinct_source_ownership_and_original_offsets` | turbo-dom-layout | (same) | verbatim | ok | ok | FAILED | ok |
| `text_source_clusters<tests/text_source_clusters.rs>::atom_cloning_and_positioning_preserve_real_nested_text_source_clusters` | turbo-dom-layout | (same) | verbatim | ok | ok | FAILED | ok |
| `text_source_clusters<tests/text_source_clusters.rs>::caret_selects_actual_combining_cluster_edges_without_inventing_an_internal_offset` | turbo-dom-layout | (same) | verbatim | ok | ok | FAILED | ok |
| `text_source_clusters<tests/text_source_clusters.rs>::cluster_geometry_shares_immutable_native_ancestry_and_survives_snapshot_drop` | turbo-dom-layout | (same) | verbatim | ok | ok | FAILED | ok |
| `text_source_clusters<tests/text_source_clusters.rs>::existing_box_only_snapshots_keep_output_and_do_not_allocate_text_provenance` | turbo-dom-layout | (same) | verbatim | ok | ok | FAILED | ok |
| `text_source_clusters<tests/text_source_clusters.rs>::font_alias_does_not_select_a_foreign_legacy_metric_adjustment` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `text_source_clusters<tests/text_source_clusters.rs>::fractional_html_used_values_and_indent_match_chrome_before_affine_projection` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `text_source_clusters<tests/text_source_clusters.rs>::html_flex_intrinsic_measurement_preserves_physical_font_policy_and_negative_leading` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `text_source_clusters<tests/text_source_clusters.rs>::html_negative_leading_keeps_native_font_boxes_without_changing_pdf_line_height` | turbo-dom-layout | (same) | verbatim | ok | ok | FAILED | ok |
| `text_source_clusters<tests/text_source_clusters.rs>::native_cluster_advances_match_independent_chrome_ranges_with_the_same_real_font` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `text_source_clusters<tests/text_source_clusters.rs>::nonfinite_caret_input_and_unsupported_real_transforms_are_explicit` | turbo-dom-layout | (same) | verbatim | ok | ok | FAILED | ok |
| `text_source_clusters<tests/text_source_clusters.rs>::original_whitespace_unicode_and_wrapped_offsets_survive_actual_shaping` | turbo-dom-layout | (same) | verbatim | ok | ok | FAILED | ok |
| `text_source_clusters<tests/text_source_clusters.rs>::physical_font_metrics_and_leading_match_small_controlled_chrome_fonts` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `text_source_clusters<tests/text_source_clusters.rs>::physical_font_overflow_fails_before_any_source_geometry_can_be_published` | turbo-dom-layout | (same) | verbatim | ok | ok | FAILED | ok |
| `text_source_clusters<tests/text_source_clusters.rs>::same_source_font_fallback_runs_choose_the_nearby_actual_edge` | turbo-dom-layout | (same) | verbatim | ok | ok | FAILED | ok |
| `text_source_clusters<tests/text_source_clusters.rs>::source_character_work_has_a_bound_even_below_the_byte_budget` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `text_source_clusters<tests/text_source_clusters.rs>::source_clusters_preserve_native_shaper_direction_without_claiming_bidi_itemization` | turbo-dom-layout | (same) | verbatim | ok | ok | FAILED | ok |
| `text_source_clusters<tests/text_source_clusters.rs>::source_enabled_layout_preserves_the_existing_public_paragraph_output` | turbo-dom-layout | (same) | verbatim | ok | ok | FAILED | ok |
| `text_source_clusters<tests/text_source_clusters.rs>::source_metadata_preserves_distinct_baselines_with_identical_public_run_identity` | turbo-dom-layout | (same) | verbatim | ok | ok | FAILED | ok |
| `text_source_clusters<tests/text_source_clusters.rs>::source_scoped_cluster_geometry_retains_original_ownership_and_full_query_results` | turbo-dom-layout | (same) | verbatim | ok | ok | FAILED | ok |
| `text_source_clusters<tests/text_source_clusters.rs>::text_geometry_errors_preserve_missing_unplaced_and_invalid_input_distinctions` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `text_source_clusters<tests/text_source_clusters.rs>::text_snapshot_errors_keep_the_original_source_and_viewport_validation` | turbo-dom-layout | (same) | verbatim | ok | ok | FAILED | ok |
| `text_source_clusters<tests/text_source_clusters.rs>::whitespace_only_source_bytes_have_a_bound_before_native_layout_capture` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::emit::fonts::collection_identity_tests::shared_collection_faces_get_separate_pdf_resources` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::emit::tounicode::tests::cmap_blocks_are_limited_to_one_hundred_entries` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::emit::tounicode::tests::reverse_mapping_uses_original_font_and_handles_stripped_cmap` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::emit::tounicode::tests::unicode_destinations_use_utf16_and_codes_are_sorted_and_unique` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::html_layout::tests::base64_and_data_uri_decode_a_png_header` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::html_layout::tests::base64_and_data_uri_decode_a_png_header` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::html_layout::tests::base64_decode_handles_plus_slash_and_rejects_invalid` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::html_layout::tests::base64_decode_handles_plus_slash_and_rejects_invalid` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::html_layout::tests::block_children_of_a_directive_box_is_empty` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::html_layout::tests::block_children_of_a_directive_box_is_empty` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::html_layout::tests::box_sizing_inherit_chains_from_the_html_root` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::html_layout::tests::box_sizing_inherit_chains_from_the_html_root` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::html_layout::tests::collect_style_css_empty_without_styles` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::html_layout::tests::collect_style_css_empty_without_styles` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::html_layout::tests::inline_block_data_cid_is_reported` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::html_layout::tests::inline_block_data_cid_is_reported` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::html_layout::tests::layout_boxes_ignores_untagged_and_inline_flattened` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::html_layout::tests::layout_boxes_ignores_untagged_and_inline_flattened` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::html_layout::tests::layout_boxes_reports_placed_data_cid_geometry` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::html_layout::tests::layout_boxes_reports_placed_data_cid_geometry` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::html_layout::tests::layout_boxes_reports_transformed_bounding_box` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::html_layout::tests::layout_boxes_reports_transformed_bounding_box` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::html_layout::tests::layout_boxes_sizes_a_data_uri_image_by_intrinsic_aspect` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::html_layout::tests::layout_boxes_sizes_a_data_uri_image_by_intrinsic_aspect` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::html_layout::tests::non_data_img_src_is_not_collected` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::html_layout::tests::non_data_img_src_is_not_collected` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::html_layout::tests::parse_html_keeps_body_and_skips_jinja` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::html_layout::tests::parse_html_keeps_body_and_skips_jinja` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::image::format_tests::gif_is_probed_but_not_decoded` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::image::format_tests::gif_is_probed_but_not_decoded` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::image::format_tests::probe_gif_reads_logical_screen_size` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::image::format_tests::probe_gif_reads_logical_screen_size` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::image::format_tests::probe_gif_too_short_for_screen_descriptor_is_none` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::image::format_tests::probe_gif_too_short_for_screen_descriptor_is_none` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::image::format_tests::probe_webp_extended_reads_canvas_size` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::image::format_tests::probe_webp_extended_reads_canvas_size` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::image::format_tests::probe_webp_lossless_reads_packed_size` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::image::format_tests::probe_webp_lossless_reads_packed_size` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::image::format_tests::probe_webp_lossless_rejects_bad_signature` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::image::format_tests::probe_webp_lossless_rejects_bad_signature` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::image::format_tests::probe_webp_lossy_full_len_wrong_start_code_is_none` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::image::format_tests::probe_webp_lossy_full_len_wrong_start_code_is_none` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::image::format_tests::probe_webp_lossy_reads_frame_size` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::image::format_tests::probe_webp_lossy_reads_frame_size` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::image::format_tests::probe_webp_lossy_rejects_bad_start_code` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::image::format_tests::probe_webp_lossy_rejects_bad_start_code` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::image::format_tests::probe_webp_unknown_chunk_is_none` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::image::format_tests::probe_webp_unknown_chunk_is_none` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::image::format_tests::sniff_recognizes_gif_and_webp` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::image::format_tests::sniff_recognizes_gif_and_webp` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::block::tests::bottom_margin_collapse_requires_a_provably_zero_minimum_height` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::block::tests::bottom_margin_collapse_requires_a_provably_zero_minimum_height` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::block::tests::first_inflow_child_is_none_for_a_non_block_box` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::block::tests::first_inflow_child_is_none_for_a_non_block_box` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::block::tests::line_strut_prefers_line_height_then_face_then_fallback` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::block::tests::line_strut_prefers_line_height_then_face_then_fallback` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::boxgen::tests::br_becomes_an_inline_line_break` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::boxgen::tests::br_becomes_an_inline_line_break` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::boxgen::tests::is_br_matches_only_the_br_tag` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::boxgen::tests::is_br_matches_only_the_br_tag` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::boxgen::tests::original_unit_policy_reuse_releases_stale_intrinsic_and_measure_caches` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::boxgen::tests::original_unit_policy_reuse_releases_stale_intrinsic_and_measure_caches` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::boxgen::tests::reusable_box_tree_resets_intrinsic_measurement_and_placement_policy` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::boxgen::tests::reusable_box_tree_resets_intrinsic_measurement_and_placement_policy` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::flex::coverage_tests::abs_item_inset_anchors_fully_auto_horizontal_axis_to_start` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::flex::coverage_tests::abs_item_inset_anchors_fully_auto_horizontal_axis_to_start` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::flex::coverage_tests::align_self_maps_every_keyword` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::flex::coverage_tests::align_self_maps_every_keyword` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::flex::coverage_tests::dim_and_edge_approximate_calc_by_its_percentage` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::flex::coverage_tests::dim_and_edge_approximate_calc_by_its_percentage` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::flex::coverage_tests::flex_natural_row_and_column` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::flex::coverage_tests::flex_natural_row_and_column` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::flex::coverage_tests::flex_shorthand_basis_resolves_every_form` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::flex::coverage_tests::flex_shorthand_basis_resolves_every_form` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::flex::coverage_tests::flex_shorthand_grow_shrink` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::flex::coverage_tests::flex_shorthand_grow_shrink` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::flex::coverage_tests::grid_area_quoting_helpers` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::flex::coverage_tests::grid_area_quoting_helpers` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::flex::coverage_tests::grid_line_parses_span_index_and_pair` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::flex::coverage_tests::grid_line_parses_span_index_and_pair` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::flex::coverage_tests::inline_block_atomic_measured_in_line` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::flex::coverage_tests::inline_block_atomic_measured_in_line` | verbatim | ok | ok | FAILED | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::flex::coverage_tests::item_basis_percent_and_width_fallback` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::flex::coverage_tests::item_basis_percent_and_width_fallback` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::flex::coverage_tests::item_inset_percentage_edge` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::flex::coverage_tests::item_inset_percentage_edge` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::flex::coverage_tests::justify_content_grid_is_none_when_unset` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::flex::coverage_tests::justify_content_grid_is_none_when_unset` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::flex::coverage_tests::justify_content_value_maps_every_keyword` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::flex::coverage_tests::justify_content_value_maps_every_keyword` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::flex::coverage_tests::justify_items_maps_grid_inline_alignment` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::flex::coverage_tests::justify_items_maps_grid_inline_alignment` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::flex::coverage_tests::justify_self_maps_every_keyword` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::flex::coverage_tests::justify_self_maps_every_keyword` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::flex::coverage_tests::min_content_replaced_image_and_directive` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::flex::coverage_tests::min_content_replaced_image_and_directive` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::flex::coverage_tests::track_of_maps_content_keywords` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::flex::coverage_tests::track_of_maps_content_keywords` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::flex::coverage_tests::track_parsers_percent_and_auto_branches` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::flex::coverage_tests::track_parsers_percent_and_auto_branches` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::imgsize::coverage_tests::live_degenerate_intrinsic_dimensions_stay_finite_and_honor_css_minimums` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::imgsize::coverage_tests::live_degenerate_intrinsic_dimensions_stay_finite_and_honor_css_minimums` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::imgsize::coverage_tests::resolve_dim_handles_px_percent_calc_and_auto` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::imgsize::coverage_tests::resolve_dim_handles_px_percent_calc_and_auto` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::inline::inline_advances::tests::numeric_spacing_without_source_metadata_preserves_the_next_atomic_placement` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::inline::inline_advances::tests::numeric_spacing_without_source_metadata_preserves_the_next_atomic_placement` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::inline::native_text_advance::tests::physical_glyph_advances_match_headed_chrome_without_layout_rounding` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::inline::native_text_advance::tests::physical_glyph_advances_match_headed_chrome_without_layout_rounding` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::inline::native_text_advance::tests::signed_conversion_spacing_and_original_utf16_preserve_shaper_semantics` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::inline::native_text_advance::tests::signed_conversion_spacing_and_original_utf16_preserve_shaper_semantics` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::inline::tests::atom_margin_top_honors_vertical_align` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::inline::tests::atom_margin_top_honors_vertical_align` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::inline::tests::forced_break_makes_strut_tall_lines` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::inline::tests::forced_break_makes_strut_tall_lines` | verbatim | ok | ok | FAILED | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::inline::tests::shared_collection_indexes_do_not_merge_text_runs` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::inline::tests::shared_collection_indexes_do_not_merge_text_runs` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::inline::tests::trailing_break_adds_no_extra_line` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::inline::tests::trailing_break_adds_no_extra_line` | verbatim | ok | ok | FAILED | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::inline::tests::valign_shift_factors` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::inline::tests::valign_shift_factors` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::inline::tests::words_wrap_when_they_exceed_the_line` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::inline::tests::words_wrap_when_they_exceed_the_line` | verbatim | ok | ok | FAILED | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::inline_boxes::defensive_contract_tests::unmatched_closing_marker_does_not_create_an_element_fragment` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::inline_boxes::defensive_contract_tests::unmatched_closing_marker_does_not_create_an_element_fragment` | verbatim | ok | ok | FAILED | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::value::affine::tests::native_angle_units_and_unsupported_lists_preserve_explicit_absence` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::value::affine::tests::native_angle_units_and_unsupported_lists_preserve_explicit_absence` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::value::affine::tests::native_css_matrix_composition_keeps_full_precision_and_function_order` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::value::affine::tests::native_css_matrix_composition_keeps_full_precision_and_function_order` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::value::border_longhand_tests::border_width_longhand_overrides_shorthand_width` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::value::border_longhand_tests::border_width_longhand_overrides_shorthand_width` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::value::box_shadow_tests::color_defaults_to_currentcolor_and_zero_is_a_valid_length` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::value::box_shadow_tests::color_defaults_to_currentcolor_and_zero_is_a_valid_length` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::value::box_shadow_tests::first_layer_wins_and_none_or_partial_yields_none` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::value::box_shadow_tests::first_layer_wins_and_none_or_partial_yields_none` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::value::box_shadow_tests::inset_keyword_and_leading_color_both_parse` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::value::box_shadow_tests::inset_keyword_and_leading_color_both_parse` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::value::box_shadow_tests::parses_offsets_blur_spread_and_color` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::value::box_shadow_tests::parses_offsets_blur_spread_and_color` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::value::calc_aspect_tests::aspect_ratio_forms` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::value::calc_aspect_tests::aspect_ratio_forms` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::value::calc_aspect_tests::calc_addition_and_leading_percent_offset` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::value::calc_aspect_tests::calc_addition_and_leading_percent_offset` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::value::calc_aspect_tests::calc_mixing_percent_and_px_becomes_calc_and_resolves` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::value::calc_aspect_tests::calc_mixing_percent_and_px_becomes_calc_and_resolves` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::value::calc_aspect_tests::calc_with_unsupported_operator_or_bad_term_is_none` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::value::calc_aspect_tests::calc_with_unsupported_operator_or_bad_term_is_none` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::value::calc_aspect_tests::pure_px_calc_stays_px_pure_percent_stays_pct` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::value::calc_aspect_tests::pure_px_calc_stays_px_pure_percent_stays_pct` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::value::gradient_tests::angle_direction_and_two_stops` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::value::gradient_tests::angle_direction_and_two_stops` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::value::gradient_tests::every_side_and_corner_direction_maps_to_its_angle` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::value::gradient_tests::every_side_and_corner_direction_maps_to_its_angle` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::value::gradient_tests::explicit_percent_stops_and_even_spread_of_the_middle` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::value::gradient_tests::explicit_percent_stops_and_even_spread_of_the_middle` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::value::gradient_tests::nested_rgb_commas_dont_split_stops_and_non_linear_is_none` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::value::gradient_tests::nested_rgb_commas_dont_split_stops_and_non_linear_is_none` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::value::gradient_tests::to_side_keywords_map_to_angles_and_default_is_to_bottom` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::value::gradient_tests::to_side_keywords_map_to_angles_and_default_is_to_bottom` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::value::gradient_tests::unbalanced_parens_yield_no_gradient` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::value::gradient_tests::unbalanced_parens_yield_no_gradient` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::value::gradient_tests::unknown_direction_keyword_is_not_a_direction` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::value::gradient_tests::unknown_direction_keyword_is_not_a_direction` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::value::transform_tests::angle_units_radians_and_bare_number` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::value::transform_tests::angle_units_radians_and_bare_number` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::value::transform_tests::matrix_and_none` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::value::transform_tests::matrix_and_none` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::value::transform_tests::rotate_builds_a_rotation_matrix` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::value::transform_tests::rotate_builds_a_rotation_matrix` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::value::transform_tests::scale_and_combined_translate_scale` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::value::transform_tests::scale_and_combined_translate_scale` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::value::transform_tests::skew_functions_shear_the_linear_part` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::value::transform_tests::skew_functions_shear_the_linear_part` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::value::transform_tests::transform_origin_maps_keywords_and_lengths` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::value::transform_tests::transform_origin_maps_keywords_and_lengths` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::value::transform_tests::translate_keeps_length_percentage_offsets` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::value::transform_tests::translate_keeps_length_percentage_offsets` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::value::transform_tests::unknown_function_contributes_nothing` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::value::transform_tests::unknown_function_contributes_nothing` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout::value::transform_tests::zero_translate_arg_and_trailing_token` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout::value::transform_tests::zero_translate_arg_and_trailing_token` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout_snapshot::fixed_containing::tests::transform_admission_recognizes_identity_and_ignores_invalid_css` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout_snapshot::fixed_containing::tests::transform_admission_recognizes_identity_and_ignores_invalid_css` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout_snapshot::text::tests::native_capture_never_wraps_layout_ids_or_exceeds_the_glyph_budget` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout_snapshot::text::tests::native_capture_never_wraps_layout_ids_or_exceeds_the_glyph_budget` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout_snapshot::text_budget_contracts::native_layout_identity_exhaustion_cannot_publish_partial_text_capture` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout_snapshot::text_budget_contracts::native_layout_identity_exhaustion_cannot_publish_partial_text_capture` | verbatim | ok | ok | FAILED | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout_snapshot::text_ranges::native_capture_loss_contracts::missing_actual_source_glyphs_fail_closed_instead_of_publishing_partial_ranges` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout_snapshot::text_ranges::native_capture_loss_contracts::missing_actual_source_glyphs_fail_closed_instead_of_publishing_partial_ranges` | verbatim | ok | ok | FAILED | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout_snapshot::text_units::tests::original_units_preserve_lones_and_scalar_defaults_without_aliasing_sources` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout_snapshot::text_units::tests::original_units_preserve_lones_and_scalar_defaults_without_aliasing_sources` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::layout_snapshot::text_units::tests::unknown_duplicate_mismatched_and_excessive_original_inputs_fail_closed` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::layout_snapshot::text_units::tests::unknown_duplicate_mismatched_and_excessive_original_inputs_fail_closed` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::stamp_font::tests::codes_are_two_byte_be_pairs_matching_shaped_glyph_count` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `turbo_html2pdf_core<src/lib.rs>::stamp_font::tests::embeds_a_real_type0_font_no_base14` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `turbo_html2pdf_core<src/lib.rs>::stamp_font::tests::step1_spike::probe_a_no_catalog_load_mem_succeeds` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `turbo_html2pdf_core<src/lib.rs>::stamp_font::tests::structural_render_passes_qpdf_check` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `turbo_html2pdf_core<src/lib.rs>::stamp_font::tests::walk_closure_skips_a_dangling_reference` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `turbo_html2pdf_core<src/lib.rs>::stamp_font::tests::walk_closure_stops_at_an_already_seen_id` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `turbo_html2pdf_core<src/lib.rs>::style::cascade::dimension_hint_tests::presentational_dimensions_ignore_invalid_values_and_preserve_zero_and_percentages` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::style::cascade::dimension_hint_tests::presentational_dimensions_ignore_invalid_values_and_preserve_zero_and_percentages` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::style::cascade::iteration_tests::iteration_yields_every_pair_in_name_order_and_round_trips` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::style::cascade::iteration_tests::iteration_yields_every_pair_in_name_order_and_round_trips` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::style::quad_shorthand::tests::complete_quad_grammar_is_validated_before_expansion` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::style::quad_shorthand::tests::complete_quad_grammar_is_validated_before_expansion` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::style::quad_shorthand::tests::quad_values_keep_nested_functions_and_reject_invalid_counts` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::style::quad_shorthand::tests::quad_values_keep_nested_functions_and_reject_invalid_counts` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::template::regions::tests::explicit_pair_field_codes_desugar` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::template::regions::tests::extracts_both_regions_and_desugars_codes` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::template::regions::tests::no_region_is_a_noop` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::template::regions::tests::non_self_closing_field_code_is_left_alone` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::template::regions::tests::region_without_close_tag_is_left_in_body` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::template::regions::tests::self_closing_end_classifies_tags` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::text::font::html_metric_policy_contracts::duplicate_native_shaping_results_do_not_double_account_shared_cache_payloads` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::text::font::html_metric_policy_contracts::duplicate_native_shaping_results_do_not_double_account_shared_cache_payloads` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::text::font::html_metric_policy_contracts::mac_legacy_policy_and_unmodified_platform_metrics_use_the_same_real_font_tables` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::text::font::html_metric_policy_contracts::mac_legacy_policy_and_unmodified_platform_metrics_use_the_same_real_font_tables` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::text::font::tests::describe_prefers_typographic_family_name_id_16` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::text::font::tests::describe_prefers_typographic_family_name_id_16` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::text::font::tests::describe_reads_family_weight_italic` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::text::font::tests::describe_reads_family_weight_italic` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::text::font::tests::face_count_valid_and_garbage` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::text::font::tests::face_count_valid_and_garbage` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::text::font::tests::native_collection_name_decoding_accepts_unicode_and_rejects_other_legacy_encodings` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::text::font::tests::native_collection_name_decoding_accepts_unicode_and_rejects_other_legacy_encodings` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::text::font::tests::oversized_real_shaping_results_are_returned_without_persistent_memoization` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::text::font::tests::oversized_real_shaping_results_are_returned_without_persistent_memoization` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::text::font::tests::platform_family_names_decode_real_legacy_font_tables_on_every_host` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::text::font::tests::platform_family_names_decode_real_legacy_font_tables_on_every_host` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::text::font::tests::public_vec_constructor_keeps_its_original_allocation` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::text::font::tests::public_vec_constructor_keeps_its_original_allocation` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::text::font::tests::shared_collection_retains_distinct_faces_and_original_font_results` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::text::font::tests::shared_collection_retains_distinct_faces_and_original_font_results` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::text::font::tests::shared_shape_cache_releases_churn_without_changing_native_results` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::text::font::tests::shared_shape_cache_releases_churn_without_changing_native_results` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::text::font::tests::with_family_retags_same_program` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::text::font::tests::with_family_retags_same_program` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::text::font::utf16::tests::original_units_follow_actual_host_fallback_glyph_tables` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::text::font::utf16::tests::original_units_follow_actual_host_fallback_glyph_tables` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::text::font::utf16::tests::original_units_keep_real_glyphs_and_distinct_replacement_cache_keys` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::text::font::utf16::tests::original_units_keep_real_glyphs_and_distinct_replacement_cache_keys` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::text::font::utf16::tests::raw_unit_cache_churn_and_oversize_release_payloads_without_changing_glyphs` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::text::font::utf16::tests::raw_unit_cache_churn_and_oversize_release_payloads_without_changing_glyphs` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::text::font::utf16::tests::repeated_raw_shape_insertion_does_not_double_count_shared_payload` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::text::font::utf16::tests::repeated_raw_shape_insertion_does_not_double_count_shared_payload` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::text::font::utf16::tests::valid_scalar_chunks_keep_shaper_offsets_around_distinct_original_lones` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::text::font::utf16::tests::valid_scalar_chunks_keep_shaper_offsets_around_distinct_original_lones` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::text::registry::tests::alias_generics_maps_generic_to_installed_family` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::text::registry::tests::alias_generics_maps_generic_to_installed_family` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::text::registry::tests::html_system_generics_select_real_menlo_with_platform_only_family_records` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::text::registry::tests::html_system_generics_select_real_menlo_with_platform_only_family_records` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::text::registry::tests::load_font_file_covers_ext_read_and_parse` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::text::registry::tests::load_font_file_covers_ext_read_and_parse` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::text::registry::tests::load_system_fonts_walks_dirs_and_aliases` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::text::registry::tests::load_system_fonts_walks_dirs_and_aliases` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::text::registry::tests::loaded_collection_shares_program_without_changing_registration_order` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::text::registry::tests::loaded_collection_shares_program_without_changing_registration_order` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::text::registry::tests::native_collection_fallback_uses_selected_physical_face_not_css_alias` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::text::registry::tests::native_collection_fallback_uses_selected_physical_face_not_css_alias` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::text::registry::tests::native_glyph_fallback_cache_is_bounded_and_mutation_detaches_registry_clones` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::text::registry::tests::native_glyph_fallback_cache_is_bounded_and_mutation_detaches_registry_clones` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::text::registry::tests::native_mac_serif_policy_selects_real_faces_without_changing_pdf_or_explicit_families` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::text::registry::tests::native_mac_serif_policy_selects_real_faces_without_changing_pdf_or_explicit_families` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::text::registry::tests::platform_only_fonts_require_html_opt_in_and_keep_caller_priority` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::text::registry::tests::platform_only_fonts_require_html_opt_in_and_keep_caller_priority` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_core<src/lib.rs>::text::registry::tests::system_font_dirs_lists_platform_dirs` | turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::text::registry::tests::system_font_dirs_lists_platform_dirs` | verbatim | ok | ok | ok | ok |
| `turbo_html2pdf_mcp<src/lib.rs>::tests::append_pdf_glues_documents` | turbo-html2pdf-core | (same) | stays | ok | - | - | - |
| `turbo_html2pdf_mcp<src/lib.rs>::tests::base64_round_trips_arbitrary_bytes` | turbo-html2pdf-core | (same) | stays | ok | - | - | - |
| `turbo_html2pdf_mcp<src/lib.rs>::tests::check_template_reports_header_footer` | turbo-html2pdf-core | (same) | stays | ok | - | - | - |
| `turbo_html2pdf_mcp<src/lib.rs>::tests::errors_are_clean` | turbo-html2pdf-core | (same) | stays | ok | - | - | - |
| `turbo_html2pdf_mcp<src/lib.rs>::tests::initialize_advertises_tools` | turbo-html2pdf-core | (same) | stays | ok | - | - | - |
| `turbo_html2pdf_mcp<src/lib.rs>::tests::notifications_get_no_reply` | turbo-html2pdf-core | (same) | stays | ok | - | - | - |
| `turbo_html2pdf_mcp<src/lib.rs>::tests::render_emits_a_pdf_with_base64_and_page_count` | turbo-html2pdf-core | (same) | stays | ok | - | - | - |
| `turbo_html2pdf_mcp<src/lib.rs>::tests::render_honors_full_option_surface` | turbo-html2pdf-core | (same) | stays | ok | - | - | - |
| `turbo_html2pdf_mcp<src/lib.rs>::tests::unknown_method_and_tool_are_reported` | turbo-html2pdf-core | (same) | stays | ok | - | - | - |
| `types<tests/types.rs>::compile_error_display_includes_line` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `types<tests/types.rs>::diagnostics_push_and_is_empty` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `types<tests/types.rs>::effective_depth_default_and_custom` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `types<tests/types.rs>::lint_construct_and_equality` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `types<tests/types.rs>::program_is_send_and_sync` | turbo-html2pdf-core | (same) | stays (split file) | ok | ok | ok | ok |
| `types<tests/types.rs>::render_error_display_includes_line` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `types<tests/types.rs>::span_from_range_and_none` | turbo-dom-layout | (same) | split | ok | ok | ok | ok |
| `watermark<tests/watermark.rs>::custom_angle_changes_the_rotation_matrix` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `watermark<tests/watermark.rs>::custom_text_watermark_word_color_angle_opacity` | turbo-html2pdf-core | (same) | stays | - | - | ok | ok |
| `watermark<tests/watermark.rs>::default_emit_has_no_watermark_state` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `watermark<tests/watermark.rs>::draft_text_watermark_is_present_and_faded` | turbo-html2pdf-core | (same) | stays | - | - | ok | ok |
| `watermark<tests/watermark.rs>::image_watermark_centered_via_shared_resolver` | turbo-html2pdf-core | (same) | stays | - | - | ok | ok |
| `watermark<tests/watermark.rs>::image_watermark_paints_behind_body` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `watermark<tests/watermark.rs>::image_watermark_tiled_draws_many_copies` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `watermark<tests/watermark.rs>::qpdf_check_watermarked_pdf_when_available` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `watermark<tests/watermark.rs>::text_watermark_is_vertically_centred_on_the_page` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `watermark<tests/watermark.rs>::text_watermark_lands_centred_on_the_page_at_any_angle` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `watermark<tests/watermark.rs>::text_watermark_paints_behind_body` | turbo-html2pdf-core | (same) | stays | - | - | ok | ok |
| `watermark<tests/watermark.rs>::unresolved_image_watermark_paints_nothing` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `watermark<tests/watermark.rs>::watermark_emit_is_byte_deterministic` | turbo-html2pdf-core | (same) | stays | ok | ok | ok | ok |
| `wikipedia_layout_harness<tests/wikipedia_layout_harness.rs>::absolute_child_in_flex_column_keeps_its_own_width` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `wikipedia_layout_harness<tests/wikipedia_layout_harness.rs>::bfc_container_contains_its_float_but_a_plain_block_does_not` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `wikipedia_layout_harness<tests/wikipedia_layout_harness.rs>::border_radius_50pct_is_half_the_shorter_side` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `wikipedia_layout_harness<tests/wikipedia_layout_harness.rs>::centered_row_flex_lays_children_side_by_side` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `wikipedia_layout_harness<tests/wikipedia_layout_harness.rs>::checked_toggle_reveals_menu` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `wikipedia_layout_harness<tests/wikipedia_layout_harness.rs>::display_none_menu_does_not_paint` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `wikipedia_layout_harness<tests/wikipedia_layout_harness.rs>::empty_box_honors_min_height` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `wikipedia_layout_harness<tests/wikipedia_layout_harness.rs>::float_wraps_content_in_a_later_sibling_section` | turbo-dom-layout | (same) | verbatim | ok | ok | FAILED | ok |
| `wikipedia_layout_harness<tests/wikipedia_layout_harness.rs>::grid_item_contains_its_floated_child` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `wikipedia_layout_harness<tests/wikipedia_layout_harness.rs>::grid_template_shorthand_sets_fixed_tracks` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `wikipedia_layout_harness<tests/wikipedia_layout_harness.rs>::hidden_dropdown_menu_does_not_paint` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `wikipedia_layout_harness<tests/wikipedia_layout_harness.rs>::html_class_gates_descendant_rule` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `wikipedia_layout_harness<tests/wikipedia_layout_harness.rs>::infobox_floats_right_via_no_space_media_query` | turbo-dom-layout | (same) | verbatim | ok | ok | FAILED | ok |
| `wikipedia_layout_harness<tests/wikipedia_layout_harness.rs>::infobox_survives_is_list_collapse_rule` | turbo-dom-layout | (same) | verbatim | ok | ok | FAILED | ok |
| `wikipedia_layout_harness<tests/wikipedia_layout_harness.rs>::media_max_width_calc_breakpoint` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `wikipedia_layout_harness<tests/wikipedia_layout_harness.rs>::navbox_nested_subgroup_has_height` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `wikipedia_layout_harness<tests/wikipedia_layout_harness.rs>::navbox_rows_stack_without_overlap` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `wikipedia_layout_harness<tests/wikipedia_layout_harness.rs>::nested_vector_three_columns_place_side_by_side` | turbo-dom-layout | (same) | verbatim | ok | ok | FAILED | ok |
| `wikipedia_layout_harness<tests/wikipedia_layout_harness.rs>::page_sections_stack_without_overlap` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `wikipedia_layout_harness<tests/wikipedia_layout_harness.rs>::paragraph_lines_widen_below_the_float` | turbo-dom-layout | (same) | verbatim | ok | ok | FAILED | ok |
| `wikipedia_layout_harness<tests/wikipedia_layout_harness.rs>::supports_grid_applies_and_negation_dropped` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `wikipedia_layout_harness<tests/wikipedia_layout_harness.rs>::table_grows_to_a_wide_spanning_cell` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `wikipedia_layout_harness<tests/wikipedia_layout_harness.rs>::text_wraps_beside_a_right_float` | turbo-dom-layout | (same) | verbatim | ok | ok | FAILED | ok |
| `wikipedia_layout_harness<tests/wikipedia_layout_harness.rs>::three_column_grid_places_columns_side_by_side` | turbo-dom-layout | (same) | verbatim | ok | ok | ok | ok |
| `xref<tests/xref.rs>::anchor_name_rides_the_fragment` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `xref<tests/xref.rs>::emits_named_destination_and_goto_link` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `xref<tests/xref.rs>::external_and_empty_links_are_not_internal` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `xref<tests/xref.rs>::internal_link_href_rides_the_fragment` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `xref<tests/xref.rs>::no_xref_markup_writes_no_annotations` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `xref<tests/xref.rs>::output_is_deterministic` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |
| `xref<tests/xref.rs>::qpdf_accepts_the_document` | turbo-html2pdf-core | (same) | stays | ok | ok | - | - |

## turbo-surf-raster -> turbo-dom-raster

| old id | new id | category | result |
|---|---|---|---|
| `screenshot<tests/screenshot.rs>::background_image_div_paints_supplied_bytes` | `screenshot<tests/screenshot.rs>::background_image_div_paints_supplied_bytes` | verbatim | ok |
| `screenshot<tests/screenshot.rs>::border_radius_renders_rounded_rect` | `screenshot<tests/screenshot.rs>::border_radius_renders_rounded_rect` | verbatim | ok |
| `screenshot<tests/screenshot.rs>::box_shadow_paints_a_soft_offset_shadow_behind_a_card` | `screenshot<tests/screenshot.rs>::box_shadow_paints_a_soft_offset_shadow_behind_a_card` | verbatim | ok |
| `screenshot<tests/screenshot.rs>::css_in_js_inline_style_blocks_apply` | `screenshot<tests/screenshot.rs>::css_in_js_inline_style_blocks_apply` | verbatim | ok |
| `screenshot<tests/screenshot.rs>::css_transform_rotates_a_box_about_its_centre` | `screenshot<tests/screenshot.rs>::css_transform_rotates_a_box_about_its_centre` | verbatim | ok |
| `screenshot<tests/screenshot.rs>::datauri_mask_image_icon_paints_without_a_fetched_asset` | `screenshot<tests/screenshot.rs>::datauri_mask_image_icon_paints_without_a_fetched_asset` | verbatim | ok |
| `screenshot<tests/screenshot.rs>::default_viewport_is_1280x800` | `screenshot<tests/screenshot.rs>::default_viewport_is_1280x800` | verbatim | ok |
| `screenshot<tests/screenshot.rs>::delazy_populates_missing_img_src_for_layout` | `screenshot<tests/screenshot.rs>::delazy_populates_missing_img_src_for_layout` | verbatim | ok |
| `screenshot<tests/screenshot.rs>::format_dispatch_matches_direct` | `screenshot<tests/screenshot.rs>::format_dispatch_matches_direct` | verbatim | ok |
| `screenshot<tests/screenshot.rs>::full_page_grows_height_to_content` | `screenshot<tests/screenshot.rs>::full_page_grows_height_to_content` | verbatim | ok |
| `screenshot<tests/screenshot.rs>::image_urls_extracts_lazy_and_responsive_attrs` | `screenshot<tests/screenshot.rs>::image_urls_extracts_lazy_and_responsive_attrs` | verbatim | ok |
| `screenshot<tests/screenshot.rs>::image_urls_lists_img_and_background_refs` | `screenshot<tests/screenshot.rs>::image_urls_lists_img_and_background_refs` | verbatim | ok |
| `screenshot<tests/screenshot.rs>::linear_gradient_background_paints_left_to_right` | `screenshot<tests/screenshot.rs>::linear_gradient_background_paints_left_to_right` | verbatim | ok |
| `screenshot<tests/screenshot.rs>::mask_image_icon_paints_tint_through_svg_alpha` | `screenshot<tests/screenshot.rs>::mask_image_icon_paints_tint_through_svg_alpha` | verbatim | ok |
| `screenshot<tests/screenshot.rs>::noscript_styles_are_inert` | `screenshot<tests/screenshot.rs>::noscript_styles_are_inert` | verbatim | ok |
| `screenshot<tests/screenshot.rs>::png_falls_back_to_placeholder_without_bytes` | `screenshot<tests/screenshot.rs>::png_falls_back_to_placeholder_without_bytes` | verbatim | ok |
| `screenshot<tests/screenshot.rs>::png_paints_supplied_image_bytes_not_a_placeholder` | `screenshot<tests/screenshot.rs>::png_paints_supplied_image_bytes_not_a_placeholder` | verbatim | ok |
| `screenshot<tests/screenshot.rs>::renders_png_with_magic_and_size` | `screenshot<tests/screenshot.rs>::renders_png_with_magic_and_size` | verbatim | ok |
| `screenshot<tests/screenshot.rs>::renders_svg_with_boxes_and_glyph_paths` | `screenshot<tests/screenshot.rs>::renders_svg_with_boxes_and_glyph_paths` | verbatim | ok |
| `screenshot<tests/screenshot.rs>::screenshot_renders_fractionally_positioned_one_pixel_border` | `screenshot<tests/screenshot.rs>::screenshot_renders_fractionally_positioned_one_pixel_border` | verbatim | ok |
| `screenshot<tests/screenshot.rs>::svg_embeds_supplied_image_as_data_uri` | `screenshot<tests/screenshot.rs>::svg_embeds_supplied_image_as_data_uri` | verbatim | ok |
| `screenshot<tests/screenshot.rs>::svg_source_image_is_rasterized_and_painted` | `screenshot<tests/screenshot.rs>::svg_source_image_is_rasterized_and_painted` | verbatim | ok |
| `screenshot<tests/screenshot.rs>::system_fonts_opt_in_renders` | `screenshot<tests/screenshot.rs>::system_fonts_opt_in_renders` | verbatim | ok |
| `screenshot<tests/screenshot.rs>::z_index_controls_stacking_paint_order` | `screenshot<tests/screenshot.rs>::z_index_controls_stacking_paint_order` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::canvas_ops::tests::font_shorthand_tolerates_whitespace_runs` | `turbo_dom_raster<src/lib.rs>::canvas_ops::tests::font_shorthand_tolerates_whitespace_runs` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::canvas_ops::tests::rgba_accepts_fractional_and_percentage_alpha` | `turbo_dom_raster<src/lib.rs>::canvas_ops::tests::rgba_accepts_fractional_and_percentage_alpha` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::image_paint::tests::add_data_uri_images_recovers_an_inline_svg_mask` | `turbo_dom_raster<src/lib.rs>::image_paint::tests::add_data_uri_images_recovers_an_inline_svg_mask` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::image_paint::tests::base64_decode_is_the_inverse_of_encode` | `turbo_dom_raster<src/lib.rs>::image_paint::tests::base64_decode_is_the_inverse_of_encode` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::image_paint::tests::base64_matches_known_vectors` | `turbo_dom_raster<src/lib.rs>::image_paint::tests::base64_matches_known_vectors` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::image_paint::tests::decode_data_uri_handles_base64_and_plain_svg` | `turbo_dom_raster<src/lib.rs>::image_paint::tests::decode_data_uri_handles_base64_and_plain_svg` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::image_paint::tests::decodes_a_simple_svg_to_rgba` | `turbo_dom_raster<src/lib.rs>::image_paint::tests::decodes_a_simple_svg_to_rgba` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::image_paint::tests::inline_svg_becomes_img_carrying_size_and_class` | `turbo_dom_raster<src/lib.rs>::image_paint::tests::inline_svg_becomes_img_carrying_size_and_class` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::image_paint::tests::inline_svg_nested_is_depth_matched_and_unbalanced_left_alone` | `turbo_dom_raster<src/lib.rs>::image_paint::tests::inline_svg_nested_is_depth_matched_and_unbalanced_left_alone` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::image_paint::tests::percent_decode_and_css_unescape` | `turbo_dom_raster<src/lib.rs>::image_paint::tests::percent_decode_and_css_unescape` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::image_paint::tests::url_token_scan_takes_data_uris_whole_ignoring_inner_delimiters` | `turbo_dom_raster<src/lib.rs>::image_paint::tests::url_token_scan_takes_data_uris_whole_ignoring_inner_delimiters` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::paint_canvas::tests::clamps_zero_size_to_valid_png` | `turbo_dom_raster<src/lib.rs>::paint_canvas::tests::clamps_zero_size_to_valid_png` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::paint_canvas::tests::clear_rect_punches_transparent` | `turbo_dom_raster<src/lib.rs>::paint_canvas::tests::clear_rect_punches_transparent` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::paint_canvas::tests::distinct_op_lists_differ` | `turbo_dom_raster<src/lib.rs>::paint_canvas::tests::distinct_op_lists_differ` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::paint_canvas::tests::empty_ops_yield_blank_png_at_size` | `turbo_dom_raster<src/lib.rs>::paint_canvas::tests::empty_ops_yield_blank_png_at_size` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::paint_canvas::tests::fill_rect_paints_its_colour` | `turbo_dom_raster<src/lib.rs>::paint_canvas::tests::fill_rect_paints_its_colour` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::paint_canvas::tests::fill_text_renders_non_blank` | `turbo_dom_raster<src/lib.rs>::paint_canvas::tests::fill_text_renders_non_blank` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::paint_canvas::tests::parse_ops_edge_cases` | `turbo_dom_raster<src/lib.rs>::paint_canvas::tests::parse_ops_edge_cases` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::paint_png::tests::fractional_border_clips_to_the_viewport_and_ignores_empty_boxes` | `turbo_dom_raster<src/lib.rs>::paint_png::tests::fractional_border_clips_to_the_viewport_and_ignores_empty_boxes` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::paint_png::tests::fractional_single_pixel_border_preserves_partial_coverage` | `turbo_dom_raster<src/lib.rs>::paint_png::tests::fractional_single_pixel_border_preserves_partial_coverage` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::style_extract::tests::delazy_does_not_clobber_an_existing_inline_style` | `turbo_dom_raster<src/lib.rs>::style_extract::tests::delazy_does_not_clobber_an_existing_inline_style` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::style_extract::tests::delazy_fills_src_and_reveals_lazy_image` | `turbo_dom_raster<src/lib.rs>::style_extract::tests::delazy_fills_src_and_reveals_lazy_image` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::style_extract::tests::delazy_keeps_a_real_data_uri_when_no_lazy_url` | `turbo_dom_raster<src/lib.rs>::style_extract::tests::delazy_keeps_a_real_data_uri_when_no_lazy_url` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::style_extract::tests::delazy_replaces_a_placeholder_data_uri_src_with_the_lazy_url` | `turbo_dom_raster<src/lib.rs>::style_extract::tests::delazy_replaces_a_placeholder_data_uri_src_with_the_lazy_url` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::style_extract::tests::delazy_replaces_a_spacer_src_with_the_lazy_url` | `turbo_dom_raster<src/lib.rs>::style_extract::tests::delazy_replaces_a_spacer_src_with_the_lazy_url` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::style_extract::tests::delazy_survives_gt_inside_attribute_value` | `turbo_dom_raster<src/lib.rs>::style_extract::tests::delazy_survives_gt_inside_attribute_value` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::style_extract::tests::delazy_takes_largest_srcset_and_skips_when_src_present` | `turbo_dom_raster<src/lib.rs>::style_extract::tests::delazy_takes_largest_srcset_and_skips_when_src_present` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::style_extract::tests::empty_when_no_styles` | `turbo_dom_raster<src/lib.rs>::style_extract::tests::empty_when_no_styles` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::style_extract::tests::ignores_similar_tag_names` | `turbo_dom_raster<src/lib.rs>::style_extract::tests::ignores_similar_tag_names` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::style_extract::tests::image_urls_decode_html_entities` | `turbo_dom_raster<src/lib.rs>::style_extract::tests::image_urls_decode_html_entities` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::style_extract::tests::image_urls_extracts_img_and_background_skipping_data` | `turbo_dom_raster<src/lib.rs>::style_extract::tests::image_urls_extracts_img_and_background_skipping_data` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::style_extract::tests::image_urls_in_css_extracts_external_backgrounds` | `turbo_dom_raster<src/lib.rs>::style_extract::tests::image_urls_in_css_extracts_external_backgrounds` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::style_extract::tests::image_urls_prefers_lazy_over_a_placeholder_src` | `turbo_dom_raster<src/lib.rs>::style_extract::tests::image_urls_prefers_lazy_over_a_placeholder_src` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::style_extract::tests::image_urls_survive_gt_inside_attribute_value` | `turbo_dom_raster<src/lib.rs>::style_extract::tests::image_urls_survive_gt_inside_attribute_value` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::style_extract::tests::pulls_head_and_body_styles_in_order` | `turbo_dom_raster<src/lib.rs>::style_extract::tests::pulls_head_and_body_styles_in_order` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::style_extract::tests::strip_non_visual_removes_script_and_style_source` | `turbo_dom_raster<src/lib.rs>::style_extract::tests::strip_non_visual_removes_script_and_style_source` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::style_extract::tests::strip_non_visual_spares_similar_tags_and_unclosed` | `turbo_dom_raster<src/lib.rs>::style_extract::tests::strip_non_visual_spares_similar_tags_and_unclosed` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::style_extract::tests::strip_noscript_removes_tag_and_body` | `turbo_dom_raster<src/lib.rs>::style_extract::tests::strip_noscript_removes_tag_and_body` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::style_extract::tests::stylesheet_hrefs_decode_html_entities` | `turbo_dom_raster<src/lib.rs>::style_extract::tests::stylesheet_hrefs_decode_html_entities` | verbatim | ok |
| `turbo_surf_raster<src/lib.rs>::style_extract::tests::stylesheet_hrefs_extracts_rel_stylesheet` | `turbo_dom_raster<src/lib.rs>::style_extract::tests::stylesheet_hrefs_extracts_rel_stylesheet` | verbatim | ok |

## S3a — selector engine Chrome fixes (turbo-dom `consol/s3a-selectors`)

No test moves. Test ids below are `binary<source>::test`. "legacy" means
`--features selector-dual` with `TURBO_DOM_SELECTOR_ENGINE=legacy`, where callers get the
frozen B-main engine's answers. That column lists the exact expected-to-fail-in-legacy set
of Gate A. Every other turbo-dom and turbo-dom-browser-env test has the same result as at
S0 in plain, legacy and new builds.

| test id | category | plain / new | legacy |
|---|---|---|---|
| `turbo_dom<src/lib.rs>::rtdom::query::tests::pseudo_optional_readonly_readwrite_selected` | changed@S3a (control `cascade_selectors_chrome156.json` `read-only-read-write`) | ok | FAILED (B-main `:read-only` = the `readonly` attribute) |
| `cascade_selectors_chrome<tests/cascade_selectors_chrome.rs>::selectors_match_chrome156` | added (the Chrome 156 control) | ok | FAILED (every bug-list entry disagrees) |
| `turbo_dom<src/lib.rs>::rtdom::query::tests::s3a_selector_fixes_edge_cases` | added (edges of the fixes the control does not reach) | ok | FAILED |
| `turbo_dom<src/lib.rs>::rtdom::query_frozen::frozen_tests::*` (59) | dual builds only: B-main's query.rs tests run against the frozen engine | ok | ok |
| `turbo_dom<src/lib.rs>::rtdom::selector_dual::tests::star_bar_p_divergence_is_logged_again_on_the_second_identical_qsa_call` | dual builds only (gate unit test) | ok | ok |

## S2 (DOM swap): new, adapted and renamed tests

S2 moves no test. Every S1 test id keeps its result in every configuration (per-test
TSVs compared against the S1 tips: html2pdf `--workspace`, core `--all-features` /
`--no-default-features` / `--features svg`; turbo-dom-layout default / `--all-features` /
`--no-default-features` / `--features svg`; turbo-dom-raster; turbo-dom +
turbo-dom-browser-env), except the one rename below. The temporary layout features are
gone, so turbo-dom-layout's `tests/print_color.rs` (formerly compiled only with
`print-color`) now runs in every configuration.

Categories: **new** — added in S2; **adapted** — same assertions on the new API;
**adapted (dual-input)** — the test's Node literals / rendered flow are styled through
BOTH inputs (legacy `style_tree` and the Tree input) with an equality assertion, then
continue on the Tree result (`tests/common/mod.rs` `style_nodes` / `style_flow`) — kept
until S4 removes the legacy input; **adapted (mechanical)** — compile-only follow-up of a
type change (`StyledElement.source`, `StyledText`, `Tag::Marker`, `Page.semantics`).

| crate | test (binary::name) | category |
|---|---|---|
| turbo-dom | `turbo_dom<src/lib.rs>::rtdom::tree::tree_access::tests::{parse_with_default_is_parse_and_scripting_makes_noscript_raw_text, character_data_reads_buffer_overlay_created_and_lone_surrogate_text, local_attributes_ignore_prefixes_and_cover_every_storage}` | new |
| turbo-dom-layout | `tree_input_equivalence<tests/tree_input_equivalence.rs>::{tree_input_matches_the_legacy_node_input, shell_styling_records_sources_and_skips_non_content, markers_classify_by_local_name_and_never_strip}` | new (the S2 gate) |
| turbo-dom-layout | `layout_options<tests/layout_options.rs>::{internal_links_get_their_own_box_only_when_asked, recorded_sources_cover_elements_texts_rows_and_the_id_range, markers_are_opaque_kept_and_unmatched_by_type_selectors}` | new |
| turbo-dom-layout | `print_color<tests/print_color.rs>::{cmyk_is_rejected_unless_accepted, style_and_layout_entries_accept_cmyk_only_when_asked}` | new |
| turbo-dom-layout | `print_color<tests/print_color.rs>::{cmyk_function_parses_percentages_and_fractions, cmyk_function_rejects_malformed_input, cmyk_components_clamp_out_of_range}` | adapted (runtime `accept_cmyk`; all configurations) |
| turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::markup::tests::classifier_marks_rejects_and_shells_stay_plain`, `::html_layout::tests::t_elements_are_unknown_inline_elements_on_the_html_path` | new |
| turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::html_layout::tests::block_children_of_a_directive_box_is_empty` -> `::block_children_of_a_marker_box_is_empty` | adapted (renamed) |
| turbo-dom-layout | `image<tests/image.rs>` (8 tests via `styled_img` / Node literals), `style<tests/style.rs>::root_pseudo_matches_html_element`, `layout_snapshot<tests/layout_snapshot.rs>` (6 tests) | adapted (dual-input) |
| turbo-dom-layout | `layout_block`, `layout_boxgen`, `layout_flex`, `layout_grid`, `layout_table`, `text_source_clusters`, in-source `boxgen`/`flex`/`block` tests | adapted (mechanical) |
| turbo-html2pdf-core | `turbo_html2pdf_core<src/lib.rs>::equivalence_tests::tree_input_matches_the_legacy_node_input` | new (the S2 gate) |
| turbo-html2pdf-core | `turbo_html2pdf_core<src/lib.rs>::markers::tests::markers_round_trip_every_directive`, `endnotes<tests/endnotes.rs>::slots_met_after_the_list_are_dropped_only_at_walked_levels` | new |
| turbo-html2pdf-core | `fixtures`, `pdf_a`, `emit`, `layout`, `style`, `conformance_flags` (render_nodes users) | adapted (dual-input `style_flow`) |
| turbo-html2pdf-core | `image`, `svg`, `layout_snapshot` (Node literals) | adapted (dual-input `style_nodes`) |
| turbo-html2pdf-core | `nodes` (`Flow::to_nodes`), `endnotes` (rewrite on a `Flow`), `xref` + `conformance_flags` (pages from `render_pages`, links/anchors from `Page::semantics`) | adapted |
| turbo-html2pdf-core | `layout_block`, `layout_boxgen`, `layout_flex`, `layout_fragment`, `layout_snapshot`, `svg_viewport_snapshot`, `native_inline_coverage`, `emit` (`Tag::Marker`), `append`, `print_color`, `stamp`, `image`, `encrypt`, `encryption_roundtrip`, `watermark` (`Page.semantics`) | adapted (mechanical) |

## S3b — the layout cascade on turbo-dom's selector engine (turbo-dom `consol/s3b-unify`, turbo-html2pdf `consol/s3b-unify`)

No test moves. Test ids are `binary<source>::test`. "legacy" means a `selector-dual`
build with `TURBO_DOM_SELECTOR_ENGINE=legacy` (the cascade gets the old html2pdf
parser + matcher's answers, the live Tree the frozen S3a engine's); that column is the
exact expected-to-fail-in-legacy set of Gate A. Every other turbo-dom, turbo-dom-browser-env,
turbo-dom-layout, turbo-dom-raster and turbo-html2pdf test has the S2 result in plain,
legacy and new builds (all configurations: layout default / svg / no-default / every
feature; html2pdf workspace, core every feature / no-default / svg).

| test id | category | plain / new | legacy |
|---|---|---|---|
| turbo-dom-layout `cascade_selectors_chrome<tests/cascade_selectors_chrome.rs>::html_path_cascade_matches_chrome156` | added (the end-to-end HTML-path Chrome 156 control: every S3a selector case + every S3b cascade case) | ok | FAILED (209 disagreements with Chrome: the old engine's quirk classes) |
| turbo-html2pdf `style<tests/style.rs>::unknown_pseudo_class_is_ignored` | changed@S3b (control `cascade_selectors_chrome156.json` cascade case `unknown-pseudo-class`): an unknown pseudo-class drops the rule | ok | FAILED (old: `a:totallyunknown` matched every `a`) |
| turbo-dom-layout `tree_input_equivalence<tests/tree_input_equivalence.rs>::tree_input_matches_the_legacy_node_input` | adapted@S3b: the `svg-xlink-href` document styles with `svg a` / `use` / `[id=g]` instead of `[href]` (the legacy forest has no attribute namespaces; the Tree input matches `[href]` like Chrome — control `attribute-namespace`) | ok | ok |
| turbo-dom `cascade_selectors_chrome<tests/cascade_selectors_chrome.rs>::selectors_match_chrome156` | extended: also asserts `parse_for_cascade` validity, `match_for_cascade` (live navigator) and, for every S3b cascade case, a minimal cascade with `specificity()` | ok | ok |
| turbo-dom `turbo_dom<src/lib.rs>::rtdom::query::cascade::tests::*` (6: `validity_follows_chrome_stylesheet_rules`, `specificity_follows_selectors_4`, `pseudo_elements_never_match_and_name_their_target`, `slotted_matches_only_through_a_slot_aware_navigator`, `cascade_only_pseudos_match_like_chrome`, `position_cache_builds_each_sibling_list_once`) | added (cascade-mode API edges) | ok | ok |
| turbo-dom `turbo_dom<src/lib.rs>::rtdom::query_frozen::frozen_tests::*` (56) | dual builds only, deleted with the gate: the S3a tip's query.rs tests against the re-frozen engine | — | ok |
| turbo-dom `turbo_dom<src/lib.rs>::rtdom::selector_dual::tests::a_repeated_qsa_is_compared_again_from_each_engines_own_cache` | dual builds only, deleted with the gate (renamed from `star_bar_p_divergence_is_logged_again_on_the_second_identical_qsa_call`: no live selector diverges from the S3a engine) | — | ok |

The S3a dual-only tests (`query_frozen::frozen_tests::*`, the `selector_dual` unit test)
and `scripts/test_selector_dual_check.py` were deleted with the gate in the last S3b
commit; the Chrome controls stay.

## S4 — turbo-surf's live layout on the live Tree (turbo-dom `consol/s4-surfmode`, turbo-surf `consol/s4-copylayer`)

turbo-surf's live-tree copier and composed styler are replaced by turbo-dom-layout's
live-DOM input (`live`), and surf's pure CSSOM-View modules move to turbo-dom-layout's
`cssom_view`. Test ids are `binary<source>::test`; turbo-surf ids are in
`turbo_surf_render<src/lib.rs>` unless stated. Every other turbo-surf test id keeps its
step-2 result (`cargo test --workspace`, the step-2 run = the S3a-consumer run + the
budget tests); every turbo-dom, turbo-dom-browser-env, turbo-dom-layout (default /
`--all-features` / `--no-default-features` / `--features svg`) and turbo-dom-raster
test id keeps its S3b result.

Categories: **moved (adapted@S4)** — the in-file test moved with its module; the
assertions are unchanged, the fixture is built with `live::testing` (light-DOM capture,
rules pre-matched with turbo-dom's engine through `LiveNavigator`) instead of surf's
copier; **adapted@S4** — same assertions through the copy-free capture (surf's test-only
`capture_tree` is now the composed capture of the light tree); **changed@S4** — the
`selector()` support change, backed by the Chrome 156 control
`turbo-surf harness/detection/fixtures/css-supports-chrome156.json`.

| old id (turbo-surf) | new id | category |
|---|---|---|
| `live_layout_metrics::tests::{transformed_border_rect_and_untransformed_client_offset_metrics_are_distinct, actual_exclusion_returns_no_rects_but_visible_empty_inline_is_explicitly_unsupported}` | turbo-dom-layout `turbo_dom_layout<src/lib.rs>::cssom_view::metrics::tests::(same)` | moved (adapted@S4) |
| `live_layout_metrics::tests::{offset_extents_ignore_empty_continuation_points_but_keep_zero_width_lines, cssom_bounding_union_ignores_empty_fragments_unless_all_are_empty}` | turbo-dom-layout `::cssom_view::metrics::tests::(same)` | moved (verbatim) |
| `live_layout_visibility::tests::opacity_and_visibility_filters_follow_resolved_flat_styles` | turbo-dom-layout `::cssom_view::visibility::tests::(same)` | moved (adapted@S4) |
| `live_layout_visibility::tests::relevance_uses_the_half_viewport_proximity_margin` | turbo-dom-layout `::cssom_view::visibility::tests::(same)` | moved (verbatim) |
| `live_layout_hit::tests::actual_inverse_transform_excludes_axis_bounds_and_degenerate_boxes` | turbo-dom-layout `::cssom_view::hit::tests::(same)` | moved (verbatim) |
| `live_layout_range::tests::actual_child_comments_and_astral_units_keep_native_dom_boundaries` | turbo-dom-layout `::cssom_view::range::tests::(same)` | moved (verbatim) |
| `live_layout_scroll::tests::scroll_extent_uses_actual_overflow_padding_margins_transforms_and_nested_clips` | turbo-surf `live_layout::tests::(same)` | stays in surf (its Chrome control is surf's `scroll-layout-chrome155.json`), adapted@S4 |
| `live_layout::tests::actual_tree_ids_attributes_and_script_style_nodes_survive_the_native_copy` | `live_layout::tests::actual_tree_ids_attributes_and_script_style_nodes_survive_the_native_capture` | adapted@S4 (renamed; `<html>` checked through its identity path instead of the copied Node) |
| `live_layout::tests::source_copy_refuses_unimplemented_namespace_and_excess_depth_or_text` | `live_layout::tests::source_capture_refuses_unimplemented_namespace_and_excess_depth_or_text` | adapted@S4 (renamed; the composed test capture reports the scope graph's depth budget first, as a browsing-context capture does — the walk's own depth error is pinned by `live_layout_budget_tests`) |
| `live_layout::tests::{mutations_move_the_same_source_id_and_snapshot_version_changes, ua_origin_does_not_outrank_universal_author_css_and_scripts_are_not_stripped, ordered_live_css_rules_do_not_move_conditionals_after_following_rules}`, `live_layout_neutral::tests::captured_css_dependency_graph_can_be_replayed_offline` | (same) | adapted@S4 (layout through the composed live path) |
| `live_layout_zoom::tests::factors_follow_the_computed_value_and_effective_zoom_multiplies` | (same) | adapted@S4 (`apply` also collects the source styles) |
| `live_layout_supports::tests::native_declarations_validate_values_and_nested_boolean_grammar` | (same) | changed@S4 (adds `selector()` cases) |
| `shadow_layout<tests/shadow_layout.rs>::genuine_css_supports_overloads_native_grammar_selectors_and_private_coercion_match_chrome` | (same) | changed@S4 (control `css-supports-chrome156.json`, 22 new `selector()` cases; replaces the Chrome 155 fixture) |

New tests:

| crate | test | what |
|---|---|---|
| turbo-surf | `live_layout_budget_tests::*` (10) | budget characterization, committed before S4 on the copier and passing unchanged after it |
| turbo-surf | `live_layout_zoom::tests::the_hook_walk_collects_flat_parents_tags_and_effective_zoom` | the post-cascade walk's style collection |
| turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::live::tests::*` (13) | capture paths/budgets/errors, composed providers, quirks/body, SVG viewports, pre-matched cascade ranking, inline override, text sources, hook, error pass-through |
| turbo-dom | `turbo_dom<src/lib.rs>::rtdom::query::cascade::tests::supports_selectors_follow_chrome` | `parse_for_supports` against the Chrome 156 `selector()` control |

## S5b — release readiness (turbo-dom `consol/s5b-release`)

No behaviour change: every cargo result equals 9cac873's except the tests below.

| crate | test | category |
|---|---|---|
| turbo-dom-browser-env | `test_support_api<tests/test_support_api.rs>::test_support_constants_are_the_shipped_files` | changed@S5b (67 files instead of 49; `FILES` sorted; JSON controls with lone UTF-16 surrogates must fail `serde_json` only for that reason) |
| turbo-dom-browser-env | `test_support_api<tests/test_support_api.rs>::custom_element_lifecycle_pairs_are_exported` | new: the 9 probe/control pairs of turbo-surf's `tests/custom_elements.rs`, each control recorded by Chrome 155 for that probe's sha256 |
| scripts (python) | `scripts/test_publish_crates.py` (10) | new: release helper (sparse-index paths, already-published guard, index polling, `[patch]` stripping, publish order vs the dependency graph) |
| tools/parity (python) | `tools/parity/test_parity.py` (13) | new: `compare`/`record`/`run` + committed checkpoints and expected-diff files |

The `turbo-dom-layout --no-default-features` baseline-red set (44 tests, turbo-html2pdf#28)
is now a committed list, `scripts/baseline-red/turbo-dom-layout-no-default-features.tsv`,
checked exactly by `scripts/check-baseline-red.py` in CI. (P5b: 43 —
`layout_flex::deeply_nested_flex_is_not_exponential` passes since turbo-html2pdf#29: the
body's 8px UA margin gives the font-less galley a height.)

## S5a — image layer, legacy Node input removal, fonts dedupe, lints (turbo-dom `consol/s5a-image`, turbo-html2pdf `consol/s5a-image`)

S5a moves no test between repos. Every test id not listed keeps its result in every
configuration (turbo-dom `--workspace`; turbo-dom-layout default / `--all-features` /
`--no-default-features` / `--features svg`; turbo-dom-raster; turbo-html2pdf
`--workspace`, core `--all-features` / `--no-default-features` / `--features svg`; per-test
TSVs against the S4 tips). Categories: **retired@S5** — the test's only job was comparing
the legacy owned-`Node` input with the Tree input, and that input is gone (the parity
harness and the TreeView tests cover it); **adapted@S5** — same assertions, the input
built as a turbo-dom `Tree` (turbo-dom-layout `test_support::literal` tree literals
instead of `turbo_dom_layout::Node`; `tests/common` `style_nodes` / `style_flow` no
longer also style through `style_tree` and assert equality) or the font fixtures read
from turbo-dom-layout's `assets/fonts` through `test_support`; **new**.

| crate | test (binary::name) | category |
|---|---|---|
| turbo-dom-layout | `tree_input_equivalence<tests/tree_input_equivalence.rs>::tree_input_matches_the_legacy_node_input` | retired@S5 (the S2 gate) |
| turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::markup::tests::classifier_marks_rejects_and_shells_stay_plain` | retired@S5 (`markup.rs`, the html5ever -> `Node` converter, is deleted) |
| turbo-dom-layout | `tree_input_equivalence<tests/tree_input_equivalence.rs>::{shell_styling_records_sources_and_skips_non_content, markers_classify_by_local_name_and_never_strip, navigator_scopes_detached_and_shell_matching}` -> `tree_view<tests/tree_view.rs>::(same)` | adapted@S5 (moved, unchanged: they test the Tree input itself) |
| turbo-dom-layout | `image`, `layout_block`, `layout_boxgen`, `layout_flex`, `layout_grid`, `layout_snapshot`, `layout_table`, `native_block_inline`, `native_font_run_advances`, `native_iframe_sizing`, `native_inline_borders`, `native_inline_coverage`, `native_inline_spacing_coverage`, `native_range_transform_stages`, `native_serif_policy`, `original_text_units`, `style`, `svg_viewport_snapshot`, `text_source_clusters` (Node literals / legacy `parse_html` -> tree literals; `snapshot_*live_tree*` called with `literal::tree(..).input()`) | adapted@S5 |
| turbo-dom-layout | `style_matched<tests/style_matched.rs>::attribute_hints_are_explicit_candidates_overridden_by_author_rules` | adapted@S5 (`presentational_declarations(Some("img"), &attrs)`) |
| turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::html_layout::tests::{parse_html_keeps_body_and_skips_jinja, collect_style_css_empty_without_styles, t_elements_are_unknown_inline_elements_on_the_html_path}`, `::layout::boxgen` reuse-policy tests, `::layout_snapshot::text_budget_contracts::*`, `::layout_snapshot::text_ranges` tests | adapted@S5 (`parse_html_tree` / `collect_style_css_tree` / tree literals) |
| turbo-dom-layout | `image_layer<tests/image_layer.rs>::{shared_png_decode_matches_the_old_pdf_decode, png_frame_keeps_16_bit_samples_and_gray, png_budget_rejects_frames_above_the_image_crate_limit, malformed_png_decodes_to_none}` (+ with `svg`: `svg_oracle::{shared_svg_rasterizer_matches_the_old_pdf_svg_path, pixel_size_separates_css_size_from_oversample}`) | new (the S5 image-layer differential gate, PDF side) |
| turbo-dom-layout | `turbo_dom_layout<src/lib.rs>::image::format_tests::png_budgets_keep_each_consumers_limits`, `::text::registry::tests::system_registry_is_loaded_once_per_policy` | new |
| turbo-dom-raster | `turbo_dom_raster<src/lib.rs>::image_paint::shared_layer_tests::{image_matrix_decodes_and_sizes_byte_identically, every_sample_value_converts_like_to_rgba8}` | new (the S5 image-layer differential gate, raster side: old `image::load_from_memory(..).to_rgba8()` / old 1x resvg == new RGBA, old probe-of-the-PNG-re-encode == new intrinsic, over `tools/parity` image-matrix + extra images and every u8/u16 sample) |
| turbo-html2pdf-core | `turbo_html2pdf_core<src/lib.rs>::equivalence_tests::tree_input_matches_the_legacy_node_input` | retired@S5 (the S2 gate) |
| turbo-html2pdf-core | `html_drive<tests/html_drive.rs>::parse_html_returns_the_legacy_node_copy` -> `::parse_html_returns_the_flow_with_directives_as_markers`, `::flow_debug_shows_the_node_copy` -> `::flow_debug_shows_the_markup` | adapted@S5 (renamed: `parse_html` returns the `Flow`; `Flow`'s Debug shows the serialized markup) |
| turbo-html2pdf-core | `nodes`, `endnotes` (flows read back via `literal::from_tree`), `image`, `svg`, `svg_viewport_snapshot`, `layout_snapshot`, `layout_block`, `layout_boxgen`, `layout_flex`, `native_inline_coverage` (tree literals), `fixtures`, `pdf_a`, `emit`, `layout`, `style`, `conformance_flags` (`style_flow` without the legacy comparison), every `tests/common` font user, `turbo_html2pdf_core<src/lib.rs>::emit::tounicode::tests::reverse_mapping_uses_original_font_and_handles_stripped_cmap` (Go font via `test_support::font_bytes`) | adapted@S5 |
