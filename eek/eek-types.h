/* 
 * Copyright (C) 2010-2011 Daiki Ueno <ueno@unixuser.org>
 * Copyright (C) 2010-2011 Red Hat, Inc.
 * Copyright (C) 2019 Purism, SPC
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

#ifndef EEK_TYPES_H
#define EEK_TYPES_H 1

#include <glib-object.h>

G_BEGIN_DECLS

#define I_(string) g_intern_static_string (string)

#define EEK_TYPE_POINT (eek_point_get_type ())
#define EEK_TYPE_BOUNDS (eek_bounds_get_type ())

typedef struct _EekBounds EekBounds;

typedef struct _EekboardContextService EekboardContextService;
typedef struct _ServerContextService ServerContextService;
typedef struct _LevelKeyboard LevelKeyboard;

/**
 * EekPoint:
 * @x: X coordinate of the point
 * @y: Y coordinate of the point
 *
 * 2D vertex
 */
typedef struct _EekPoint EekPoint;
struct _EekPoint
{
    /*< public >*/
    gdouble x;
    gdouble y;
};

GType     eek_point_get_type (void) G_GNUC_CONST;
EekPoint *eek_point_copy     (const EekPoint *point);
void      eek_point_free     (EekPoint       *point);
void      eek_point_rotate   (EekPoint       *point,
                              gint            angle);

/**
 * EekBounds:
 * @x: X coordinate of the top left point
 * @y: Y coordinate of the top left point
 * @width: width of the box
 * @height: height of the box
 *
 * The rectangle containing an element's bounding box.
 */
struct _EekBounds
{
    /*< public >*/
    gdouble x;
    gdouble y;
    gdouble width;
    gdouble height;
};

GType      eek_bounds_get_type (void) G_GNUC_CONST;
EekBounds *eek_bounds_copy     (const EekBounds *bounds);
void       eek_bounds_free     (EekBounds       *bounds);

struct transformation {
    gdouble origin_x;
    gdouble origin_y;
    gdouble scale;
};

G_END_DECLS
#endif  /* EEK_TYPES_H */
