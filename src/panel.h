#pragma once

#include "eek/layersurface.h"
#include "src/layout.h"
#include "src/submission.h"

// Stores the objects that the panel and its widget will refer to
struct panel_manager {
    EekboardContextService *state; // unowned
    /// Needed for instantiating the widget
    struct submission *submission; // unowned
    struct squeek_layout_state *layout;

    PhoshLayerSurface *window;
    GtkWidget *widget; // nullable

    // Those should be held in Rust
    struct wl_output *current_output;
};

struct panel_manager panel_manager_new(EekboardContextService *state, struct submission *submission, struct squeek_layout_state *layout);
