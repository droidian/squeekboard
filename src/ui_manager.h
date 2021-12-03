#ifndef UI_MANAGER__
#define UI_MANAGER__

#include <inttypes.h>

#include "eek/eek-types.h"
#include "outputs.h"
#include "main.h"

struct ui_manager;

struct ui_manager *squeek_uiman_new(void);
void squeek_uiman_set_output(struct ui_manager *uiman, struct squeek_output_handle output);
uint32_t squeek_uiman_get_perceptual_height(struct ui_manager *uiman);

struct vis_manager;

struct vis_manager *squeek_visman_new(struct squeek_state_manager *state_manager);
#endif
