#pragma once
#include <gtk/gtk.h>

// from main.h
struct sender;

// from animations.rs
struct squeek_animation_visibility_manager;

struct squeek_animation_visibility_manager *squeek_animation_visibility_manager_new(struct sender *ui_sender);

void squeek_animation_visibility_manager_send_claim_visible(struct squeek_animation_visibility_manager *animman);
void squeek_animation_visibility_manager_send_force_hide(struct squeek_animation_visibility_manager *animman);
