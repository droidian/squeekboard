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

#ifndef EEK_ELEMENT_H
#define EEK_ELEMENT_H 1

#include <glib-object.h>
#include "eek-types.h"

G_BEGIN_DECLS
#define EEK_TYPE_ELEMENT (eek_element_get_type())
G_DECLARE_DERIVABLE_TYPE (EekElement, eek_element, EEK, ELEMENT, GObject)

struct _EekElementClass
{
    /*< private >*/
    GObjectClass parent_class;
};

void         eek_element_set_name              (EekElement  *element,
                                                const gchar *name);

const gchar *eek_element_get_name              (EekElement  *element);

void         eek_element_set_bounds            (EekElement  *element,
                                                EekBounds   *bounds);

void         eek_element_get_bounds            (EekElement  *element,
                                                EekBounds   *bounds);

G_END_DECLS
#endif  /* EEK_ELEMENT_H */
