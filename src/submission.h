#ifndef __SUBMISSION_H
#define __SUBMISSION_H

#include "inttypes.h"

#include "eek/eek-types.h"

struct squeek_layout;
struct submission;

// Defined in Rust
uint8_t submission_hint_available(struct submission *self);
void submission_use_layout(struct submission *self, struct squeek_layout *layout, uint32_t time);
#endif
