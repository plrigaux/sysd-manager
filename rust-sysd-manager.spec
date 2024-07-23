# Generated by rust2rpm 26
%bcond_without check

%global crate sysd-manager

Name:           rust-sysd-manager
Version:        0.x.x
Release:        %autorelease
Summary:        GUI to manage systemd units

License:        GPLv3+
URL:            https://crates.io/crates/sysd-manager
Source:         %{crates_source}

BuildRequires:  cargo-rpm-macros >= 24
# BuildRequires:  desktop-file-install

%global _description %{expand:
A GUI to manage systemd units.}

%description %{_description}

%package     -n %{crate}
Summary:        %{summary}
# FIXME: paste output of %%cargo_license_summary here
License:        GPL-3.0-or-later
# LICENSE.dependencies contains a full license breakdown

%description -n %{crate} %{_description}

%files       -n %{crate}
%license LICENCE
%license LICENSE.dependencies
%doc README.md
# %doc meson_options.txt
%{_bindir}/sysd-manager
/usr/share/icons/hicolor/scalable/apps/io.github.plrigaux.sysd-manager.svg
/usr/share/applications/io.github.plrigaux.sysd-manager.desktop
/usr/share/glib-2.0/schemas/io.github.plrigaux.sysd-manager.gschema.xml

%prep
%autosetup -n %{crate}-%{version} -p1
%cargo_prep

%generate_buildrequires
%cargo_generate_buildrequires

%build
%cargo_build
%{cargo_license_summary}
%{cargo_license} > LICENSE.dependencies

%install
%cargo_install
install -v -Dm644 data/applications/io.github.plrigaux.sysd-manager.desktop -t %{buildroot}%{_datadir}/applications 
install -v -Dm644 data/icons/hicolor/scalable/apps/io.github.plrigaux.sysd-manager.svg -t %{buildroot}%{_datadir}/icons/hicolor/scalable/apps                           
install -v -Dm644 data/schemas/io.github.plrigaux.sysd-manager.gschema.xml -t %{buildroot}%{_datadir}/glib-2.0/schemas

%if %{with check}
%check
%cargo_test
%endif

%changelog
%autochangelog

#%post
#install -m 644 data/applications/org.tool.sysd-manager.desktop /usr/share/applications/ 
#install -m 644 data/icons/hicolor/scalable/org.tool.sysd-manager.svg /usr/share/icons/hicolor/scalable/apps/
glib-compile-schemas /usr/share/glib-2.0/schemas/