%global OrigName prefault

Name:    prefault-git
Version: 0.0.1
Release: 0%{?dist}
Summary: prefault - A leightweight tool used to pre-fault pages from often used files into memory, ahead of time.
URL:     https://x3n0m0rph59.github.io/prefault/
License: GPLv3+

# Source0: https://github.com/X3n0m0rph59/prefault.git
Source0: https://github.com/X3n0m0rph59/%{OrigName}/archive/master/%{OrigName}-master.tar.gz

BuildRoot: %{_tmppath}/%{name}-build

BuildRequires: cargo
BuildRequires: systemd

Conflicts: prefault

%global gittag master
%global debug_package %{nil}

%description
A leightweight tool used to pre-fault pages from often used files into memory, ahead of time.

%prep
%autosetup -n %{OrigName}-master

%build
cargo build --all --release --verbose

%install
%{__mkdir_p} %{buildroot}%{_mandir}/man1
%{__mkdir_p} %{buildroot}%{_unitdir}/
%{__mkdir_p} %{buildroot}%%{_sysconfdir}/%{OrigName}/%{OrigName}.conf
%{__mkdir_p} %{buildroot}%%{_sysconfdir}/%{OrigName}/cache.d
%{__mkdir_p} %{buildroot}%{_sharedstatedir}/%{OrigName}/
%{__mkdir_p} %{buildroot}%{_sharedstatedir}/%{OrigName}/snapshots/
#%{__mkdir_p} %{buildroot}%{_datarootdir}/bash-completion/completions/
#%{__mkdir_p} %{buildroot}%{_datarootdir}/zsh/site-functions/
cp -a %{_builddir}/%{name}-%{version}/support/man/prefault.1 %{buildroot}/%{_mandir}/man1/
cp -a %{_builddir}/%{name}-%{version}/support/config/prefault.conf %{buildroot}/%{_sysconfdir}/%{OrigName}/
cp -a %{_builddir}/%{name}-%{version}/support/systemd/prefault.service %{buildroot}/%{_unitdir}/
install -Dp -m 0755 %{_builddir}/%{name}-%{version}/target/release/prefault %{buildroot}%{_bindir}/prefault

%postun
%systemd_postun_with_restart %{OrigName}.service

%files
%license LICENSE
%doc %{_mandir}/man1/prefault.1.gz
%dir %{_sysconfdir}/%{OrigName}/cache.d
#%dir %{_datarootdir}/bash-completion/completions/
#%dir %{_datarootdir}/zsh/site-functions/
%config(noreplace) %{_sysconfdir}/%{OrigName}/%{OrigName}.conf
%{_bindir}/prefault
%{_unitdir}/prefault.service
%{_sharedstatedir}/%{OrigName}/
%{_sharedstatedir}/%{OrigName}/snapshots/
#%{_datarootdir}/bash-completion/completions/prefault
#%{_datarootdir}/zsh/site-functions/_prefault

%changelog
