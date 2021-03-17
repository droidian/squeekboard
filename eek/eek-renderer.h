/* 
 * Copyright (C) 2011 Daiki Ueno <ueno@unixuser.org>
 * Copyright (C) 2011 Red Hat, Inc.
 * 
 * This library is free software; you can redistribute it and/or
 * modify it under the terms of the GNU Lesser General Public License
 * as published by the Free Software Foundation; either version 2 of
 * the License, or (at your option) any later version.
 * 
 * This library is distributed in the hope that it will be useful, but
 * WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
 * Lesser General Public License for more details.
 * 
 * You should have received a copy of the GNU Lesser General Public
 * License along with this library; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA
 * 02110-1301 USA
 */

#ifndef EEK_RENDERER_H
#define EEK_RENDERER_H 1

#include <gtk/gtk.h>
#include <pango/pangocairo.h>

#include "eek-types.h"
#include "src/submission.h"

struct squeek_layout;

/// Renders LevelKayboards
/// It cannot adjust styles at runtime.
typedef struct EekRenderer
{
    PangoContext *pcontext; // owned
    GtkCssProvider *css_provider; // owned
    GtkStyleContext *view_context; // owned
    GtkStyleContext *button_context; // TODO: maybe move a copy to each button
    /// Style class for rendering the view and button CSS.
    gchar *extra_style; // owned

    // Mutable state
    /// Background extents
    gdouble allocation_width;
    gdouble allocation_height;
    gint scale_factor; /* the outputs scale factor */
    /// Coords transformation
    struct transformation widget_to_layout;
} EekRenderer;


GType            eek_renderer_get_type         (void) G_GNUC_CONST;
EekRenderer     *eek_renderer_new              (LevelKeyboard     *keyboard,
                                                PangoContext    *pcontext);
void             eek_renderer_set_allocation_size
                                               (EekRenderer     *renderer, struct squeek_layout *layout,
                                                gdouble          width,
                                                gdouble          height);
void             eek_renderer_set_scale_factor (EekRenderer     *renderer,
                                                gint             scale);

cairo_surface_t *eek_renderer_get_icon_surface(const gchar     *icon_name,
                                                gint             size,
                                                gint             scale);

void             eek_renderer_render_keyboard  (EekRenderer     *renderer, struct transformation widget_to_layout, struct submission *submission,
                                                cairo_t         *cr, LevelKeyboard *keyboard);
void
eek_renderer_free (EekRenderer        *self);

struct transformation
eek_renderer_get_transformation (EekRenderer *renderer);

G_END_DECLS
#endif  /* EEK_RENDERER_H */
