#ifndef UI_MANAGER__
#define UI_MANAGER__

#include <inttypes.h>

#include "outputs.h"

struct ui_manager;

struct ui_manager *squeek_uiman_new(void);
void squeek_uiman_set_output(struct ui_manager *uiman, struct squeek_output_handle output);
uint32_t squeek_uiman_get_perceptual_height(struct ui_manager *uiman);

#endif
