#ifndef __OUTPUTS_H
#define __OUTPUTS_H

#include "wayland-client-protocol.h"

struct squeek_outputs;
struct squeek_output_handle {
    struct wl_output *output;
    struct squeek_outputs *outputs;
};

struct squeek_outputs *squeek_outputs_new(void);
void squeek_outputs_free(struct squeek_outputs*);
void squeek_outputs_register(struct squeek_outputs*, struct wl_output *output, uint32_t id);
struct wl_output *squeek_outputs_try_unregister(struct squeek_outputs*, uint32_t id);
struct squeek_output_handle squeek_outputs_get_current(struct squeek_outputs*);
int32_t squeek_outputs_get_perceptual_width(struct squeek_outputs*, struct wl_output *output);
#endif
