#!/usr/bin/env python3

import gi
import sys
gi.require_version('Gtk', '3.0')

from gi.repository import Gtk

try:
    terminal = [("Terminal", Gtk.InputPurpose.TERMINAL)]
except AttributeError:
    print("Terminal purpose not available on this GTK version", file=sys.stderr)
    terminal = []

def new_grid(items, set_type):
    grid = Gtk.Grid(orientation='vertical', column_spacing=8, row_spacing=8)
    i = 0
    for text, value in items:
        l = Gtk.Label(label=text)
        e = Gtk.Entry(hexpand=True)
        set_type(e, value)
        grid.attach(l, 0, i, 1, 1)
        grid.attach(e, 1, i, 1, 1)
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
    ] + terminal

    hints = [
        ("OSK provided", Gtk.InputHints.INHIBIT_OSK)
    ]

    def do_activate(self):
        w = Gtk.ApplicationWindow(application=self)
        w.set_default_size (300, 500)
        notebook = Gtk.Notebook()
        def add_purpose(entry, purpose):
            entry.set_input_purpose(purpose)
        def add_hint(entry, hint):
            entry.set_input_hints(hint)
        purpose_grid = new_grid(self.purposes, add_purpose)
        hint_grid = new_grid(self.hints, add_hint)

        purpose_scroll = Gtk.ScrolledWindow()
        purpose_scroll.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        purpose_scroll.add(purpose_grid)
        notebook.append_page(purpose_scroll, Gtk.Label(label="Purposes"))
        notebook.append_page(hint_grid, Gtk.Label(label="Hints"))
        w.add(notebook)
        w.show_all()

app = App()
app.run(sys.argv)
