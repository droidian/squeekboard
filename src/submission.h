#ifndef __SUBMISSION_H
#define __SUBMISSION_H

#include "input-method-unstable-v2-client-protocol.h"
#include "virtual-keyboard-unstable-v1-client-protocol.h"
#include "eek/eek-types.h"
#include "main.h"
#include "src/ui_manager.h"

struct squeek_layout;

// Defined in Rust
uint8_t submission_hint_available(struct submission *self);
void submission_use_layout(struct submission *self, struct squeek_layout *layout, uint32_t time);
#endif
