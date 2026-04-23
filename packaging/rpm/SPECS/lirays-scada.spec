Name:           lirays-scada
Version:        %{?version}%{!?version:0.1.0}
Release:        1%{?dist}
Summary:        LiRAYS Scada Server
%global debug_package %{nil}
%global _enable_debug_packages 0
License:        Proprietary
URL:            https://lirays.com
Source0:        lirays-scada
Source1:        lirays-scada.service
Source2:        settings.default.yaml
Source3:        lirays
ExclusiveArch:  x86_64 aarch64
Requires:       systemd
Requires(pre):  shadow-utils
Requires(post): systemd
Requires(preun): systemd
Requires(postun): systemd

%description
LiRAYS Scada is a server application that provides both TCP and HTTP interfaces
for managing industrial data. This package installs the lirays-scada binary along
with a systemd service that automatically starts the server at boot time with
appropriate environment variables and directory structure.

%prep
%setup -T -c
# No source archive; binaries and assets are provided by the outer build.

%build
# Build is handled externally by Cargo; nothing to do here.

%install
rm -rf %{buildroot}
install -Dm755 %{SOURCE0} %{buildroot}%{_bindir}/lirays-scada
install -Dm755 %{SOURCE3} %{buildroot}%{_bindir}/lirays
install -Dm644 %{SOURCE1} %{buildroot}%{_unitdir}/lirays-scada.service
install -Dm644 %{SOURCE2} %{buildroot}%{_sysconfdir}/lirays-scada/settings.default.yaml
install -d %{buildroot}/var/lib/lirays-scada/data

%pre
getent group lirays >/dev/null || groupadd -r lirays
getent passwd lirays >/dev/null || \
  useradd -r -g lirays -d /var/lib/lirays-scada -s /sbin/nologin lirays

%post
chown -R lirays:lirays /var/lib/lirays-scada
chmod 755 /var/lib/lirays-scada /var/lib/lirays-scada/data
if [ ! -f /etc/lirays-scada/settings.yaml ] && [ -f /etc/lirays-scada/settings.default.yaml ]; then
  cp /etc/lirays-scada/settings.default.yaml /etc/lirays-scada/settings.yaml
  chown lirays:lirays /etc/lirays-scada/settings.yaml
  chmod 640 /etc/lirays-scada/settings.yaml
fi
%systemd_post lirays-scada.service
if [ $1 -eq 1 ]; then
  # Fresh install: enable and start service
  systemctl enable --now lirays-scada.service >/dev/null 2>&1 || true
fi

%preun
%systemd_preun lirays-scada.service

%postun
%systemd_postun_with_restart lirays-scada.service

%files
%config %{_sysconfdir}/lirays-scada/settings.default.yaml
%dir %{_sysconfdir}/lirays-scada
%{_bindir}/lirays-scada
%{_bindir}/lirays
%{_unitdir}/lirays-scada.service
%dir %attr(755,lirays,lirays) /var/lib/lirays-scada
%dir %attr(755,lirays,lirays) /var/lib/lirays-scada/data

%changelog
* Tue Mar 24 2026 Alejandro <alejandro@lirays-scada.com> - %{?version}-1
- Initial RPM release of LiRAYS Scada server
