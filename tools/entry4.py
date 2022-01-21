#!/usr/bin/env python3

import gi
import random
import sys
gi.require_version('Gtk', '4.0')
gi.require_version('GLib', '2.0')

from gi.repository import Gtk
from gi.repository import GLib


def new_grid(items, set_type):
    grid = Gtk.Grid(orientation='vertical', column_spacing=8, row_spacing=8)

    i = 0
    for text, value in items:
        label = Gtk.Label(label=text)
        label.props.margin_top = 6
        label.props.margin_start = 6
        entry = Gtk.Entry(hexpand=True)
        entry.props.margin_top = 6
        entry.props.margin_end = 6
        set_type(entry, value)
        grid.attach(label, 0, i, 1, 1)
        grid.attach(entry, 1, i, 1, 1)
        i += 1
    return grid


class App(Gtk.Application):

    purposes = [
        ("Free form", Gtk.InputPurpose.FREE_FORM),
        ("Alphabetical", Gtk.InputPurpose.ALPHA),
        ("Digits", Gtk.InputPurpose.DIGITS),
        ("Number", Gtk.InputPurpose.NUMBER),
        ("Phone", Gtk.InputPurpose.PHONE),
        ("URL", Gtk.InputPurpose.URL),
        ("E-mail", Gtk.InputPurpose.EMAIL),
        ("Name", Gtk.InputPurpose.NAME),
        ("Password", Gtk.InputPurpose.PASSWORD),
        ("PIN", Gtk.InputPurpose.PIN),
        ("Terminal", Gtk.InputPurpose.TERMINAL),
    ]

    hints = [
        ("OSK provided", Gtk.InputHints.INHIBIT_OSK)
    ]
    purpose_tick_id = 0

    def on_purpose_toggled(self, btn, entry):
        purpose = Gtk.InputPurpose.PIN if btn.get_active() else Gtk.InputPurpose.PASSWORD
        entry.set_input_purpose(purpose)

    def on_timeout(self, e):
        r = random.randint(0, len(self.purposes) - 1)
        (_, purpose) = self.purposes[r]
        print(f"Setting {purpose}")
        e.set_input_purpose(purpose)
        return True

    def on_random_enter(self, controller, entry):
        self.purpose_tick_id = GLib.timeout_add_seconds(3, self.on_timeout, entry)

    def on_random_leave(self, controller, entry):
        GLib.source_remove(self.purpose_tick_id)

    def add_random(self, grid):
        label = Gtk.Label(label="Random")
        entry = Gtk.Entry(hexpand=True)
        entry.set_input_purpose(Gtk.InputPurpose.FREE_FORM)
        grid.attach(label, 0, len(self.purposes), 1, 1)
        grid.attach(entry, 1, len(self.purposes), 1, 1)
        focus_controller = Gtk.EventControllerFocus()
        entry.add_controller(focus_controller)
        focus_controller.connect("enter", self.on_random_enter, entry)
        focus_controller.connect("leave", self.on_random_leave, entry)

    def do_activate(self):
        w = Gtk.ApplicationWindow(application=self)
        w.set_default_size(300, 500)
        notebook = Gtk.Notebook()

        def add_purpose(entry, purpose):
            entry.set_input_purpose(purpose)

        def add_hint(entry, hint):
            entry.set_input_hints(hint)
        purpose_grid = new_grid(self.purposes, add_purpose)
        self.add_random(purpose_grid)
        hint_grid = new_grid(self.hints, add_hint)

        purpose_scroll = Gtk.ScrolledWindow()
        purpose_scroll.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        purpose_scroll.set_child(purpose_grid)
        notebook.append_page(purpose_scroll, Gtk.Label(label="Purposes"))
        notebook.append_page(hint_grid, Gtk.Label(label="Hints"))
        w.set_child(notebook)
        w.present()


app = App()
app.run(sys.argv)
