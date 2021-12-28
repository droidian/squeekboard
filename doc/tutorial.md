A guide to creating layouts
===========================

This guide is based on the original Kareema's [forum post](https://forums.puri.sm/t/translations-and-virtual-touch-keyboards-tracking-localization/7669/48).

It’s long overdue to write a comprehensive guide how to add a keyboard layout from start. But unfortunately, I don’t have much time left ATM. A lot of information can be found in [this](https://forums.puri.sm/t/using-non-latin-language-on-librem-5/7103/5) thread.

So at least I will try to start writing a short how-to here and edit this post as I find the time. Hope this helps a bit - comments and corrections [welcome](https://source.puri.sm/Librem5/squeekboard/-/merge_requests/)

## Creating a new layout

Creating a layout is easy. You don't need to recompile things, just edit and test. It's easiest to start with an existing layout.

### Get one of the existing keyboard layouts

* You can get one of the keyboards from the squeekboard git repository : [https://source.puri.sm/Librem5/squeekboard](https://source.puri.sm/Librem5/squeekboard)
* The keyboard layouts are located in the subdirectory [`data/keyboards/`](https://source.puri.sm/Librem5/squeekboard/-/tree/master/data/keyboards) in the `.yaml` files
* Take a look and try to understand them :slight_smile:


### Creating the keyboard layout

* To be written: For the time being, take a look at [Using non-latin language on Librem 5](https://forums.puri.sm/t/using-non-latin-language-on-librem-5/7103/5)
* Select and enable the input source you would like to change from the Region & Language section of the device settings. Perhaps use "A user-defined custom layout" listed under Other.
* Find the correct name of the .yaml file associated with that input source. This can be found with the command 

```
gsettings get org.gnome.desktop.input-sources sources
```

The output should be something like this: `[('xkb', 'us'), ('xkb', 'de')]`
So for example “de.yaml” would be the correct name for the German keyboard layout.
If the name of your layout is not translated correctly in the list, you can fix it by adding it and recompiling Squeekboard.

There is also associated files for that layout in landscape, terminal, number, emoji mode. They can be found at something analogous to `us_wide.yaml`, `terminal/us.yaml`, `number/us.yaml`, `emoji/us.yaml`, respectively.

### Testing the layout

Copy your yaml file to `~/.local/share/squeekboard/keyboards/` for testing purposes. From there it should get picked up by squeekboard automatically.
The yaml file will overwrite the default settings for that layout. If you want to go back to default, simply remove the file.

You can also use the `test_layout` tool from the -devel package to check it for errors:

```
# squeekboard_test_layout ./mylayout.yaml
Test result: OK
```

## Contributing your changes

If you want to share your layout with the world, the best way is to submit it to the Squeekboard project. The workflow is similar to any other Gitlab-based project.

Above all, your layout should be working, be tested, not break anything, and make sense.

### Fork your own copy of squeekboard

* Best way would be to start with a fork of the squeekboard repository: Create a user account at https://source.puri.sm/, go the the squeekboard git repository, press “Fork” in the web interface. You can find further instructions [here](https://docs.gitlab.com/ee/user/project/repository/forking_workflow.html#creating-a-fork).
* Clone your fork locally with `git clone` and use the uri of your forked repo there

### Edit your keyboard and get it merged

* It may be useful to check out the [generic guide how the workflow to contribute works](https://developer.puri.sm/Librem5/Contact/Contributing.html)
* Create a branch: Name it “keyboard-layout-mylanguage” or whatever
* Checkout your branch, edit your keyboard layout and commit your changes
* Your layout **must** be correctly named, and in `data/keyboards/`.
* Your layout **must** pass the `test_layout` tool with zero problems.
* Your translation **must** be correctly named, and in `data/langs/`.
* Your layout or translation **must** be added to automatic tests. **Don’t forget to add it** to `src/resources.rs` and the layout to `tests/meson.build` (that’s for me, because I always forget it).

### Get it merged

It's always recommended to **compile and run** squeekboard before submitting your changes. This serves as a test that all is working. See instructions in the [compiling section](#compiling-and-running-squeekboard).

* Push the local changes (to the branch of your fork of squeekboard)
* Create a merge request for the branch to get your changes merged to the official squeekboard git repository

If your changes pass automated tests (CI), then the merge request will be reviewed by the maintainers, and you might be asked to change a thing or two.

## Compiling and running Squeekboard

If you want your change to become part of official Squeekboard, or if you want to add a translation of your layout name, you will have to recompile Squeekboard and test your changes there.

### Compile squeekboard

* Follow the instructions found in “Building” section of the squeekboard’s README: Running squeekboard: [https://source.puri.sm/Librem5/squeekboard/blob/master/README.md#building](https://source.puri.sm/Librem5/squeekboard/blob/master/README.md#building)

### Run squeekboard

* Follow these instructions to run squeekboard: [https://source.puri.sm/Librem5/squeekboard/blob/master/README.md#running](https://source.puri.sm/Librem5/squeekboard/blob/master/README.md#running)
* Additionally take a look at the contribution document for [testing info](HACKING.md#testing)
* You can either test it locally on your Linux system or use the [QEMU Librem 5 image](https://developer.puri.sm/Librem5/Development_Environment/Boards/emulators.html)
* To test squeekboard locally, you need phoc. Either compile that from the sources as well or use the CI repository ci.puri.sm for Debian based systems:
  `deb [arch=amd64] http://ci.puri.sm/ scratch librem5`

Squeekboard can be installed from there as a Debian package, too (that’s what I often do). But beware - there be dragons! You could bork your system with these packages and you should probably disable this repository again after installing what you need - these packages are not meant for production systems (or so I heard :wink: )
