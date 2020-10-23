/* 
 * Copyright (C) 2010-2011 Daiki Ueno <ueno@unixuser.org>
 * Copyright (C) 2010-2011 Red Hat, Inc.
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */
#if !defined(__EEKBOARD_SERVICE_H_INSIDE__) && !defined(EEKBOARD_COMPILATION)
#error "Only <eekboard/eekboard-service.h> can be included directly."
#endif

#ifndef EEKBOARD_CONTEXT_SERVICE_H
#define EEKBOARD_CONTEXT_SERVICE_H 1

#include "src/submission.h"
#include "src/layout.h"

#include "virtual-keyboard-unstable-v1-client-protocol.h"
#include "text-input-unstable-v3-client-protocol.h"

G_BEGIN_DECLS

#define EEKBOARD_CONTEXT_SERVICE_PATH "/org/fedorahosted/Eekboard/Context_%d"
#define EEKBOARD_CONTEXT_SERVICE_INTERFACE "org.fedorahosted.Eekboard.Context"

#define EEKBOARD_TYPE_CONTEXT_SERVICE (eekboard_context_service_get_type())

G_DECLARE_FINAL_TYPE(EekboardContextService, eekboard_context_service, EEKBOARD, CONTEXT_SERVICE, GObject)

EekboardContextService *eekboard_context_service_new(struct squeek_layout_state *state);
void eekboard_context_service_set_submission(EekboardContextService *context, struct submission *submission);
void eekboard_context_service_set_ui(EekboardContextService *context, ServerContextService *ui);
void          eekboard_context_service_destroy (EekboardContextService *context);
LevelKeyboard *eekboard_context_service_get_keyboard(EekboardContextService *context);

void eekboard_context_service_set_keymap(EekboardContextService *context,
                                         const LevelKeyboard *keyboard);

void eekboard_context_service_set_hint_purpose(EekboardContextService *context,
                                               uint32_t hint,
                                               uint32_t purpose);
void
eekboard_context_service_use_layout(EekboardContextService *context, struct squeek_layout_state *layout, uint32_t timestamp);
G_END_DECLS
#endif  /* EEKBOARD_CONTEXT_SERVICE_H */
