use super::*;
use crate::windows_ui::toolbar;
fn secondary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    toolbar::standard(ui, label, false)
}
fn primary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    toolbar::standard(ui, label, true)
}
impl Launcher {
    pub(super) fn plugins_page(&mut self, ui: &mut egui::Ui, app: &AppInfo) {
        toolbar::style(ui);
        app_screens::experimental_banner(ui);
        if let Some((id, index)) = self.plugins.remove.clone() {
            if id == app.id {
                let mut remove = false;
                let mut cancel = false;
                let response=egui::Modal::new(egui::Id::new("remove-plugin")).frame(egui::Frame::new().fill(panel()).corner_radius(16).inner_margin(24)).show(ui.ctx(),|ui|{
                    ui.set_width(400.0);ui.heading(tr("Remove plugin?"));ui.add_space(12.0);ui.label(tr("Remove this installed extension from the app? Your projects and the original downloaded package are kept."));ui.add_space(20.0);
                    ui.horizontal(|ui|{cancel=secondary_button(ui,"Cancel").clicked();remove=danger_button(ui,"Remove plugin").clicked();});
                });
                if remove {
                    self.plugins.message = match plugins::uninstall(app, index) {
                        Ok(()) => "Plugin removed.".into(),
                        Err(error) => error,
                    };
                    self.plugins.remove = None;
                } else if cancel || response.should_close() {
                    self.plugins.remove = None;
                }
            }
        }
        ui.label(RichText::new(tr("Plugin management")).font(toolbar::font(20.0, true)));
        ui.label(
            RichText::new(tr(
                "Install extensions for this app from a GitHub release or a local file.",
            ))
            .color(muted()),
        );
        ui.add_space(18.0);
        let Some(root) = self.prefs.default_project_root.clone() else {
            ui.label(tr(
                "Choose a default project folder before installing plugins.",
            ));
            if primary_button(ui, "Choose folder").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.add_project_folder(path);
                }
            }
            return;
        };
        let base = root.join(app.name);
        egui::Frame::new().fill(mix_color(panel(),app.tint,0.04)).stroke(egui::Stroke::new(1.0_f32,border())).corner_radius(12).inner_margin(18).show(ui,|ui|{
            ui.set_width(ui.available_width());ui.label(RichText::new(tr("Master Suite integration")).font(toolbar::font(17.0,true)));
            ui.label(RichText::new(tr("Built in · Workspace folders")).size(text_size(11.0)).color(readable_app_color(app.tint)));
            ui.add_space(10.0);
            ui.label(tr("Projects, exports, assets and plugins have separate folders inside this app's workspace."));
            ui.label(RichText::new(tr(if app.id=="effectcraft"{"The export folder is applied when EffectCraft launches. Project save dialogs still depend on the app."}else{"Apps launch from Projects. Separate save/export defaults require the app to support the Master Suite integration."})).size(text_size(12.0)).color(muted()));
            ui.add_space(12.0);
            ui.horizontal_wrapped(|ui|{for folder in ["Projects","Exports","Assets","Plugins"]{if secondary_button(ui,folder).clicked(){match workspace_bridge::prepare(&root,app){Ok(_)=>reveal_project_path(&base.join(folder),false),Err(e)=>self.plugins.message=e}}}});
        });
        ui.add_space(18.0);
        let supported = plugins::supported(app.id);
        if !supported {
            egui::Frame::new().fill(panel()).corner_radius(12).inner_margin(18).show(ui,|ui|{ui.set_width(ui.available_width());ui.label(tr("Plugin installation is not supported for this app yet."));ui.label(RichText::new(tr("A compatible plugin loader must be available in the app before Master Suite can install extensions for it.")).color(muted()));});
            return;
        }
        egui::Frame::new().fill(panel()).stroke(egui::Stroke::new(1.0_f32,border())).corner_radius(12).inner_margin(16).show(ui,|ui|{
            ui.set_width(ui.available_width());
            ui.label(RichText::new(tr("Install a plugin")).font(toolbar::font(17.0,true)));
            ui.label(RichText::new(tr("Choose a local package or paste a GitHub release link.")).font(toolbar::font(12.0,false)).color(muted()));
            ui.add_space(12.0);
            ui.label(RichText::new(tr("GitHub release URL")).font(toolbar::font(12.0,true)).color(muted()));
            toolbar::field(ui,"plugin-release-url",&mut self.plugins.link,"https://github.com/owner/plugin/releases/download/…");
            ui.add_space(12.0);
            ui.horizontal_wrapped(|ui|{
                if ui.add_enabled_ui(!self.plugins.busy&&!self.plugins.link.trim().is_empty(),|ui|primary_button(ui,"Install from GitHub")).inner.clicked(){self.plugins.install(*app,root.clone(),self.plugins.link.trim().into(),false);}
                if ui.add_enabled_ui(!self.plugins.busy,|ui|secondary_button(ui,"Install from file")).inner.clicked(){if let Some(path)=rfd::FileDialog::new().add_filter(tr("Craft WebAssembly plugin"),&["wasm","zip"]).pick_file(){self.plugins.install(*app,root.clone(),path.display().to_string(),true);}}
            });ui.add_space(10.0);
            ui.label(RichText::new(tr("Accepts compiled .wasm plugins or ZIP packages. Use a current app release with plugin support; source-code archives and native Photoshop plugins are not supported.")).size(text_size(12.0)).color(muted()));
        });
        ui.add_space(16.0);
        if self.plugins.busy {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(tr("Installing plugin…"));
            });
            ui.ctx().request_repaint_after(Duration::from_millis(200));
        }
        if !self.plugins.message.is_empty() {
            ui.label(tr(&self.plugins.message));
            ui.add_space(12.0);
        }
        toolbar::divider(ui);
        ui.label(RichText::new(tr("Installed plugins")).font(toolbar::font(17.0, true)));
        ui.add_space(12.0);
        match plugins::entries(app) {
            Err(error) => {
                ui.label(tr(error));
            }
            Ok(entries) => {
                if entries.is_empty() {
                    ui.label(
                        RichText::new(tr("No plugins installed by Master Suite yet."))
                            .color(muted()),
                    );
                }
                for (index, entry) in entries.iter().enumerate() {
                    egui::Frame::new()
                        .fill(panel())
                        .stroke(egui::Stroke::new(1.0_f32, border()))
                        .corner_radius(10)
                        .inner_margin(16)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::Label::new(RichText::new(&entry.name).strong())
                                        .truncate(),
                                )
                                .on_hover_text(&entry.name);
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .add_enabled_ui(!self.plugins.busy, |ui| {
                                                secondary_button(ui, "Remove")
                                            })
                                            .inner
                                            .clicked()
                                        {
                                            self.plugins.remove = Some((app.id.into(), index));
                                        }
                                        if ui
                                            .add_enabled_ui(!self.plugins.busy, |ui| {
                                                secondary_button(
                                                    ui,
                                                    if entry.enabled {
                                                        "Disable"
                                                    } else {
                                                        "Enable"
                                                    },
                                                )
                                            })
                                            .inner
                                            .clicked()
                                        {
                                            self.plugins.message = match plugins::toggle(app, index)
                                            {
                                                Ok(()) => {
                                                    "Plugin updated. Restart the app to apply."
                                                        .into()
                                                }
                                                Err(e) => e,
                                            };
                                        }
                                    },
                                );
                            });
                            ui.label(
                                RichText::new(tr(if entry.enabled {
                                    "Enabled · loads when the app starts"
                                } else {
                                    "Disabled"
                                }))
                                .size(text_size(12.0))
                                .color(if entry.enabled {
                                    readable_app_color(app.tint)
                                } else {
                                    muted()
                                }),
                            );
                            ui.add(
                                egui::Label::new(
                                    RichText::new(entry.deployed.display().to_string())
                                        .size(text_size(11.0))
                                        .color(muted()),
                                )
                                .truncate(),
                            )
                            .on_hover_text(entry.deployed.display().to_string());
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&entry.source)
                                        .size(text_size(11.0))
                                        .color(muted()),
                                )
                                .truncate(),
                            )
                            .on_hover_text(&entry.source);
                        });
                    ui.add_space(8.0);
                }
            }
        }
    }
}
