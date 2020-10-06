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

#if !defined(__EEK_H_INSIDE__) && !defined(EEK_COMPILATION)
#error "Only <eek/eek.h> can be included directly."
#endif

#ifndef EEK_KEYBOARD_H
#define EEK_KEYBOARD_H 1

#include <glib-object.h>
#include <xkbcommon/xkbcommon.h>
#include "eek-types.h"
#include "src/layout.h"

G_BEGIN_DECLS

/// Keymap container for Rust interoperability.
struct keymap {
    uint32_t fd; // keymap formatted as XKB string
    size_t fd_len; // length of the data inside keymap_fd
};

/// Keyboard state holder
struct _LevelKeyboard {
    struct squeek_layout *layout; // owned
    struct keymap keymap; // owned
};
typedef struct _LevelKeyboard LevelKeyboard;

gchar *eek_keyboard_get_keymap(LevelKeyboard *keyboard);

LevelKeyboard*
level_keyboard_new (struct squeek_layout *layout);
void level_keyboard_free(LevelKeyboard *self);

G_END_DECLS
#endif  /* EEK_KEYBOARD_H */
