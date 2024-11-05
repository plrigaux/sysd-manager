import dbus

bus = dbus.SystemBus()

print("get_is_authenticated", bus.get_is_authenticated())

print("get_name_owner", bus.get_name_owner("org.freedesktop.systemd1"))


print("get_unix_user", bus.get_unix_user("org.freedesktop.systemd1"))

#print("list_activatable_names", bus.list_activatable_names())


#get the object
the_object = bus.get_object("org.freedesktop.systemd1", "/org/freedesktop/systemd1")
#get the interface
the_interface = dbus.Interface(the_object, "org.freedesktop.systemd1.Manager")

#call the methods and print the results
reply = the_interface.StartUnit("tiny_daemon.service", "fail")
#reply = the_interface.ListUnits()

exit(0)
"""

[2024-11-05T19:03:25Z WARN  sysd_manager::widget::unit_control_panel::controls] Action "Enabled" on unit "tiny_daemon.service" FAILED!
    "ZBusError(Variant(SignatureMismatch(Structure(Dynamic { fields: [Bool, Array(Dynamic { child: Structure(Dynamic { fields: [Str, Str, Str] }) })] }), \"`(b(sss))`\")))"
[2024-11-05T19:03:25Z INFO  sysd_manager::widget::unit_control_panel::imp] switch_ablement_state_set new false old false



dbus-send --system --print-reply --dest=org.freedesktop.systemd1 /org/freedesktop/systemd1 org.freedesktop.systemd1.Manager.StopUnit string:"tiny_daemon.service" string:"fail"

busctl --allow-interactive-authorization=TRUE call org.freedesktop.systemd1 /org/freedesktop/systemd1 org.freedesktop.systemd1.Manager StartUnit ss "tiny_daemon.service" "fail"



object = bus.get_object("org.freedesktop.Accounts", "/org/freedesktop/Accounts")

the_interface = dbus.Interface(object, "org.freedesktop.Accounts")


the_interface.CreateUser("Boris Ivanovich", "Grishenko", 1)

exit(0)
#dbus-send --system --dest=org.freedesktop.Accounts --type=method_call --print-reply /org/freedesktop/Accounts org.freedesktop.Accounts.CreateUser string:boris string:"Boris Ivanovich Grishenko" int32:1


sender = "org.freedesktop.systemd1.manage-units"

dbus_info = dbus.Interface(bus.get_object("org.freedesktop.DBus",
                                                        "/org/freedesktop/DBus/Bus", False),
                                        "org.freedesktop.DBus")


pid = dbus_info.GetConnectionUnixProcessID(sender)


polkit = dbus.Interface(dbus.SystemBus().get_object(
        "org.freedesktop.PolicyKit1",
        "/org/freedesktop/PolicyKit1/Authority", False),
                                     "org.freedesktop.PolicyKit1.Authority")

auth_response = polkit.CheckAuthorization(
            ("unix-process", {"pid": dbus.UInt32(pid, variant_level=1),
                              "start-time": dbus.UInt64(0, variant_level=1)}),
            #privilege, 
            {"AllowUserInteraction": "true"}, dbus.UInt32(1), "", timeout=600)
print(auth_response)
 
#get the object
the_object = bus.get_object("org.freedesktop.systemd1", "/org/freedesktop/systemd1")
#get the interface
the_interface = dbus.Interface(the_object, "org.freedesktop.systemd1.Manager")

#call the methods and print the results
reply = the_interface.StartUnit("tiny_daemon.service", "fail")
#reply = the_interface.ListUnits()

print(reply)


def _check_polkit_privilege(self, sender, conn, privilege):
    # Get Peer PID
    if self.dbus_info is None:
        # Get DBus Interface and get info thru that
        self.dbus_info = dbus.Interface(conn.get_object("org.freedesktop.DBus",
                                                        "/org/freedesktop/DBus/Bus", False),
                                        "org.freedesktop.DBus")
    pid = self.dbus_info.GetConnectionUnixProcessID(sender)
 
    # Query polkit
    if self.polkit is None:
        self.polkit = dbus.Interface(dbus.SystemBus().get_object(
        "org.freedesktop.PolicyKit1",
        "/org/freedesktop/PolicyKit1/Authority", False),
                                     "org.freedesktop.PolicyKit1.Authority")
 
    # Check auth against polkit; if it times out, try again
    try:
        auth_response = self.polkit.CheckAuthorization(
            ("unix-process", {"pid": dbus.UInt32(pid, variant_level=1),
                              "start-time": dbus.UInt64(0, variant_level=1)}),
            privilege, {"AllowUserInteraction": "true"}, dbus.UInt32(1), "", timeout=600)
        print(auth_response)
        (is_auth, _, details) = auth_response
    except dbus.DBusException as e:
        if e._dbus_error_name == "org.freedesktop.DBus.Error.ServiceUnknown":
            # polkitd timeout, retry
            self.polkit = None
            return self._check_polkit_privilege(sender, conn, privilege)
        else:
            # it's another error, propagate it
            raise
 
    if not is_auth:
        # Aww, not authorized :(
        print(":(")
        return False
 
    print("Successful authorization!")
    return True
"""