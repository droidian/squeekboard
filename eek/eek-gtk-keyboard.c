/*
 * Copyright (C) 2010-2011 Daiki Ueno <ueno@unixuser.org>
 * Copyright (C) 2010-2011 Red Hat, Inc.
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

/**
 * SECTION:eek-gtk-keyboard
 * @short_description: a #GtkWidget displaying #EekKeyboard
 */

#include "config.h"

#include <math.h>
#include <string.h>

#include "eek-renderer.h"
#include "eek-keyboard.h"

#include "eek-gtk-keyboard.h"

#include "eekboard/eekboard-context-service.h"
#include "src/layout.h"
#include "src/submission.h"

#define LIBFEEDBACK_USE_UNSTABLE_API
#include <libfeedback.h>

#define SQUEEKBOARD_APP_ID "sm.puri.squeekboard"

typedef struct _EekGtkKeyboardPrivate
{
    EekRenderer *renderer; // owned, nullable
    EekboardContextService *eekboard_context; // unowned reference
    struct submission *submission; // unowned reference

    struct squeek_layout_state *layout; // unowned
    LevelKeyboard *keyboard; // unowned reference; it's kept in server-context

    GdkEventSequence *sequence; // unowned reference
    LfbEvent *event;
} EekGtkKeyboardPrivate;

G_DEFINE_TYPE_WITH_PRIVATE (EekGtkKeyboard, eek_gtk_keyboard, GTK_TYPE_DRAWING_AREA)

static void
eek_gtk_keyboard_real_realize (GtkWidget      *self)
{
    gtk_widget_set_events (self,
                           GDK_EXPOSURE_MASK |
                           GDK_KEY_PRESS_MASK |
                           GDK_KEY_RELEASE_MASK |
                           GDK_BUTTON_PRESS_MASK |
                           GDK_BUTTON_RELEASE_MASK |
                           GDK_BUTTON_MOTION_MASK |
                           GDK_TOUCH_MASK);

    GTK_WIDGET_CLASS (eek_gtk_keyboard_parent_class)->realize (self);
}

static gboolean
eek_gtk_keyboard_real_draw (GtkWidget *self,
                            cairo_t   *cr)
{
    EekGtkKeyboardPrivate *priv =
        eek_gtk_keyboard_get_instance_private (EEK_GTK_KEYBOARD (self));
    GtkAllocation allocation;
    gtk_widget_get_allocation (self, &allocation);

    if (!priv->keyboard) {
        return FALSE;
    }

    if (!priv->renderer) {
        PangoContext *pcontext = gtk_widget_get_pango_context (self);

        priv->renderer = eek_renderer_new (
                    priv->keyboard,
                    pcontext);

        eek_renderer_set_allocation_size (priv->renderer,
                                          priv->keyboard->layout,
                                          allocation.width,
                                          allocation.height);
        eek_renderer_set_scale_factor (priv->renderer,
                                       gtk_widget_get_scale_factor (self));
    }

    eek_renderer_render_keyboard (priv->renderer, priv->submission, cr, priv->keyboard);
    return FALSE;
}

// Units of pixel size
static enum squeek_arrangement_kind get_type(uint32_t width, uint32_t height) {
    (void)height;
    if (width < 1080) {
        return ARRANGEMENT_KIND_BASE;
    }
    return ARRANGEMENT_KIND_WIDE;
}

static void
eek_gtk_keyboard_real_size_allocate (GtkWidget     *self,
                                     GtkAllocation *allocation)
{
    EekGtkKeyboardPrivate *priv =
        eek_gtk_keyboard_get_instance_private (EEK_GTK_KEYBOARD (self));
    uint32_t scale = (uint32_t)gtk_widget_get_scale_factor(self);
    // check if the change would switch types
    enum squeek_arrangement_kind new_type = get_type(
                (uint32_t)(allocation->width - allocation->x) * scale,
                (uint32_t)(allocation->height - allocation->y) * scale);
    if (priv->layout->arrangement != new_type) {
        priv->layout->arrangement = new_type;

        eekboard_context_service_use_layout(priv->eekboard_context, priv->layout);
    }

    if (priv->renderer)
        eek_renderer_set_allocation_size (priv->renderer,
                                          priv->keyboard->layout,
                                          allocation->width,
                                          allocation->height);

    GTK_WIDGET_CLASS (eek_gtk_keyboard_parent_class)->
        size_allocate (self, allocation);
}

static void
on_event_triggered (LfbEvent      *event,
                    GAsyncResult  *res,
                    gpointer      unused)
{
    g_autoptr (GError) err = NULL;

    if (!lfb_event_trigger_feedback_finish (event, res, &err)) {
        g_warning ("Failed to trigger feedback for '%s': %s",
                   lfb_event_get_event (event), err->message);
    }
}

static void depress(EekGtkKeyboard *self,
                    gdouble x, gdouble y, guint32 time)
{
    EekGtkKeyboardPrivate *priv = eek_gtk_keyboard_get_instance_private (self);
    if (!priv->keyboard) {
        return;
    }
    squeek_layout_depress(priv->keyboard->layout,
                          priv->submission,
                          x, y, eek_renderer_get_transformation(priv->renderer), time, self);
}

static void drag(EekGtkKeyboard *self,
                 gdouble x, gdouble y, guint32 time)
{
    EekGtkKeyboardPrivate *priv = eek_gtk_keyboard_get_instance_private (self);
    if (!priv->keyboard) {
        return;
    }
    squeek_layout_drag(eekboard_context_service_get_keyboard(priv->eekboard_context)->layout,
                       priv->submission,
                       x, y, eek_renderer_get_transformation(priv->renderer), time,
                       priv->eekboard_context, self);
}

static void release(EekGtkKeyboard *self, guint32 time)
{
    EekGtkKeyboardPrivate *priv = eek_gtk_keyboard_get_instance_private (self);
    if (!priv->keyboard) {
        return;
    }
    squeek_layout_release(eekboard_context_service_get_keyboard(priv->eekboard_context)->layout,
                          priv->submission,
                          eek_renderer_get_transformation(priv->renderer), time,
                          priv->eekboard_context, self);
}

static gboolean
eek_gtk_keyboard_real_button_press_event (GtkWidget      *self,
                                          GdkEventButton *event)
{
    if (event->type == GDK_BUTTON_PRESS && event->button == 1) {
        depress(EEK_GTK_KEYBOARD(self), event->x, event->y, event->time);
    }
    return TRUE;
}

// TODO: this belongs more in gtk_keyboard, with a way to find out which key to re-render
static gboolean
eek_gtk_keyboard_real_button_release_event (GtkWidget      *self,
                                            GdkEventButton *event)
{
    if (event->type == GDK_BUTTON_RELEASE && event->button == 1) {
        // TODO: can the event have different coords than the previous move event?
        release(EEK_GTK_KEYBOARD(self), event->time);
    }
    return TRUE;
}

static gboolean
eek_gtk_keyboard_leave_event (GtkWidget      *self,
                              GdkEventCrossing *event)
{
    if (event->type == GDK_LEAVE_NOTIFY) {
        // TODO: can the event have different coords than the previous move event?
        release(EEK_GTK_KEYBOARD(self), event->time);
    }
    return TRUE;
}


static gboolean
eek_gtk_keyboard_real_motion_notify_event (GtkWidget      *self,
                                           GdkEventMotion *event)
{
    if (event->state & GDK_BUTTON1_MASK) {
        drag(EEK_GTK_KEYBOARD(self), event->x, event->y, event->time);
    }
    return TRUE;
}

// Only one touch stream at a time allowed. Others will be completely ignored.
static gboolean
handle_touch_event (GtkWidget     *widget,
                    GdkEventTouch *event)
{
    EekGtkKeyboard        *self = EEK_GTK_KEYBOARD (widget);
    EekGtkKeyboardPrivate *priv = eek_gtk_keyboard_get_instance_private (self);

    /* For each new touch, release the previous one and record the new event
       sequence. */
    if (event->type == GDK_TOUCH_BEGIN) {
        release(self, event->time);
        priv->sequence = event->sequence;
        depress(self, event->x, event->y, event->time);
        return TRUE;
    }

    /* Only allow the latest touch point to be dragged. */
    if (event->type == GDK_TOUCH_UPDATE && event->sequence == priv->sequence) {
        drag(self, event->x, event->y, event->time);
    }
    else if (event->type == GDK_TOUCH_END || event->type == GDK_TOUCH_CANCEL) {
        // TODO: can the event have different coords than the previous update event?
        /* Only respond to the release of the latest touch point. Previous
           touches have already been released. */
        if (event->sequence == priv->sequence) {
            release(self, event->time);
            priv->sequence = NULL;
        }
    }
    return TRUE;
}

static void
eek_gtk_keyboard_real_unmap (GtkWidget *self)
{
    EekGtkKeyboardPrivate *priv =
        eek_gtk_keyboard_get_instance_private (EEK_GTK_KEYBOARD (self));

    if (priv->keyboard) {
        squeek_layout_release_all_only(
            priv->keyboard->layout,
            priv->submission,
            gdk_event_get_time(NULL));
    }

    GTK_WIDGET_CLASS (eek_gtk_keyboard_parent_class)->unmap (self);
}

static void
eek_gtk_keyboard_set_property (GObject      *object,
                               guint         prop_id,
                               const GValue *value,
                               GParamSpec   *pspec)
{
    (void)value;
    switch (prop_id) {
    default:
        G_OBJECT_WARN_INVALID_PROPERTY_ID (object, prop_id, pspec);
        break;
    }
}

static void
eek_gtk_keyboard_dispose (GObject *object)
{
    EekGtkKeyboard        *self = EEK_GTK_KEYBOARD (object);
    EekGtkKeyboardPrivate *priv = eek_gtk_keyboard_get_instance_private (self);

    if (priv->renderer) {
        eek_renderer_free(priv->renderer);
        priv->renderer = NULL;
        priv->renderer = NULL;
    }

    if (priv->keyboard) {
        squeek_layout_release_all_only(
            priv->keyboard->layout,
            priv->submission,
            gdk_event_get_time(NULL));
        priv->keyboard = NULL;
    }

    if (priv->event) {
        g_clear_object (&priv->event);
        lfb_uninit ();
    }

    G_OBJECT_CLASS (eek_gtk_keyboard_parent_class)->dispose (object);
}

static void
eek_gtk_keyboard_class_init (EekGtkKeyboardClass *klass)
{
    GtkWidgetClass *widget_class = GTK_WIDGET_CLASS (klass);
    GObjectClass *gobject_class = G_OBJECT_CLASS (klass);

    widget_class->realize = eek_gtk_keyboard_real_realize;
    widget_class->unmap = eek_gtk_keyboard_real_unmap;
    widget_class->draw = eek_gtk_keyboard_real_draw;
    widget_class->size_allocate = eek_gtk_keyboard_real_size_allocate;
    widget_class->button_press_event =
        eek_gtk_keyboard_real_button_press_event;
    widget_class->button_release_event =
        eek_gtk_keyboard_real_button_release_event;
    widget_class->motion_notify_event =
        eek_gtk_keyboard_real_motion_notify_event;
    widget_class->leave_notify_event =
        eek_gtk_keyboard_leave_event;

    widget_class->touch_event = handle_touch_event;

    gobject_class->set_property = eek_gtk_keyboard_set_property;
    gobject_class->dispose = eek_gtk_keyboard_dispose;
}

static void
eek_gtk_keyboard_init (EekGtkKeyboard *self)
{
    EekGtkKeyboardPrivate *priv = eek_gtk_keyboard_get_instance_private (EEK_GTK_KEYBOARD (self));
    g_autoptr(GError) err = NULL;

    if (lfb_init(SQUEEKBOARD_APP_ID, &err))
        priv->event = lfb_event_new ("button-pressed");
    else
        g_warning ("Failed to init libfeedback: %s", err->message);
}

static void
on_notify_keyboard (GObject              *object,
                    GParamSpec           *spec,
                    EekGtkKeyboard *self) {
    (void)spec;
    EekGtkKeyboardPrivate *priv = (EekGtkKeyboardPrivate*)eek_gtk_keyboard_get_instance_private (self);
    priv->keyboard = eekboard_context_service_get_keyboard(EEKBOARD_CONTEXT_SERVICE(object));
    if (priv->renderer) {
        eek_renderer_free(priv->renderer);
    }
    priv->renderer = NULL;
    gtk_widget_queue_draw(GTK_WIDGET(self));
}

/**
 * Create a new #GtkWidget displaying @keyboard.
 * Returns: a #GtkWidget
 */
GtkWidget *
eek_gtk_keyboard_new (EekboardContextService *eekservice,
                      struct submission *submission,
                      struct squeek_layout_state *layout)
{
    EekGtkKeyboard *ret = EEK_GTK_KEYBOARD(g_object_new (EEK_TYPE_GTK_KEYBOARD, NULL));
    EekGtkKeyboardPrivate *priv = (EekGtkKeyboardPrivate*)eek_gtk_keyboard_get_instance_private (ret);
    priv->eekboard_context = eekservice;
    priv->submission = submission;
    priv->layout = layout;
    priv->renderer = NULL;
    g_signal_connect (eekservice,
                      "notify::keyboard",
                      G_CALLBACK(on_notify_keyboard),
                      ret);
    on_notify_keyboard(G_OBJECT(eekservice), NULL, ret);
    /* TODO: this is how a compound keyboard
     * made out of a layout and a suggestion bar could start.
     * GtkBox *box = GTK_BOX(gtk_box_new(GTK_ORIENTATION_VERTICAL, 0));
    GtkEntry *fill = GTK_ENTRY(gtk_entry_new());
    gtk_box_pack_start(box, GTK_WIDGET(fill), FALSE, FALSE, 0);
    gtk_box_pack_start(box, GTK_WIDGET(ret), TRUE, TRUE, 0);
    return GTK_WIDGET(box);*/
    return GTK_WIDGET(ret);
}

/**
 * eek_gtk_keyboard_emit_feedback:
 *
 * Emit button press haptic feedback via libfeedack.
 */
void
eek_gtk_keyboard_emit_feedback (EekGtkKeyboard *self)
{
    EekGtkKeyboardPrivate *priv;

    g_return_if_fail (EEK_IS_GTK_KEYBOARD (self));

    priv = eek_gtk_keyboard_get_instance_private (EEK_GTK_KEYBOARD (self));
    if (priv->event) {
        lfb_event_trigger_feedback_async (priv->event,
                                          NULL,
                                          (GAsyncReadyCallback)on_event_triggered,
                                          NULL);
    }
}
