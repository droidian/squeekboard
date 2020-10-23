#include <gio/gio.h>

static void
call_dbus_cb (GDBusProxy *proxy,
              GAsyncResult *res,
              gpointer user_data)
{
  g_autoptr (GError) err = NULL;
  g_autoptr (GVariant) output = NULL;

  output = g_dbus_proxy_call_finish (proxy, res, &err);
  if (err) {
    g_warning ("Can't open panel %s", err->message);
  }
  g_object_unref (proxy);
}

static void
create_dbus_proxy_cb (GObject *source_object, GAsyncResult *res, char *panel)
{
  GDBusProxy *proxy;
  g_autoptr (GError) err = NULL;
  GVariantBuilder builder;
  GVariant *params[3];
  GVariant *array[1];

  proxy = g_dbus_proxy_new_for_bus_finish (res, &err);

  if (err != NULL) {
    g_warning ("Can't open panel %s: %s", panel, err->message);
    g_free (panel);
    return;
  }

  g_variant_builder_init (&builder, G_VARIANT_TYPE ("av"));
  g_variant_builder_add (&builder, "v", g_variant_new_string (""));

  array[0] = g_variant_new ("v", g_variant_new ("(sav)", panel, &builder));

  params[0] = g_variant_new_string ("launch-panel");
  params[1] = g_variant_new_array (G_VARIANT_TYPE ("v"), array, 1);
  params[2] = g_variant_new_array (G_VARIANT_TYPE ("{sv}"), NULL, 0);

  g_dbus_proxy_call (proxy,
             "Activate",
             g_variant_new_tuple (params, 3),
             G_DBUS_CALL_FLAGS_NONE,
             -1,
             NULL,
             (GAsyncReadyCallback) call_dbus_cb,
             NULL);

  g_free (panel);
}

void
popover_open_settings_panel (char *panel)
{
  g_dbus_proxy_new_for_bus (G_BUS_TYPE_SESSION,
                G_DBUS_PROXY_FLAGS_NONE,
                NULL,
                "org.gnome.ControlCenter",
                "/org/gnome/ControlCenter",
                "org.gtk.Actions",
                NULL,
                (GAsyncReadyCallback) create_dbus_proxy_cb,
                g_strdup (panel));

}
