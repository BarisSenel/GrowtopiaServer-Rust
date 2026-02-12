use crate::database::player;

pub fn build_role_menu(player: &player::Player, active_tab: &str, net_id: i32) -> String {
    let mut dialog = String::from("set_default_color|`o\n");
    dialog.push_str(&format!("embed_data|netID|{}|\n", net_id));
    dialog.push_str("add_popup_name|role_menu|\n");


    dialog.push_str("start_custom_tabs|\n");

    let f_human = if active_tab == "roleTab_human" { "1,0" } else { "0,0" };


    dialog.push_str("add_spacer|small|\n");
    dialog.push_str("add_label_with_icon|big|`wROLE SYSTEM                                        ``|left|1366|\n");
    dialog.push_str("add_textbox|`o``|left|\n");
    dialog.push_str("add_spacer|small|\n");


    match active_tab {

        _ => {

            dialog.push_str("add_textbox|`wRole: `oHuman``|left|\n");
            dialog.push_str(&format!(
                "add_textbox|`wName: `o{}``|left|\n",
                player.name
            ));
            dialog.push_str(&format!(
                "add_textbox|`wGems: `2{}``|left|\n",
                player.gems
            ));
            dialog.push_str(&format!(
                "add_textbox|`wFarming Level: `5{}``|left|\n",
                player.farmer_lvl
            ));
        }
    }

    dialog.push_str("add_textbox|`o``|left|\n");
    dialog.push_str("add_spacer|small|\n");
    dialog.push_str("end_dialog|role_menu||Back|\n");
    dialog.push_str("add_quick_exit|\n");

    dialog
}

pub fn build_profile_menu(player: &player::Player, net_id: i32) -> String {
    let mut dialog = String::from("set_default_color|`o\n");
    dialog.push_str(&format!("embed_data|netID|{}|\n", net_id));
    dialog.push_str("add_popup_name|profile_menu|\n");
    dialog.push_str("add_label_with_icon|big|`wProfile``|left|1366|\n");
    dialog.push_str("add_spacer|small|\n");
    dialog.push_str(&format!("add_textbox|`wName: `o{}``|left|\n", player.name));
    dialog.push_str(&format!("add_textbox|`wGems: `2{}``|left|\n", player.gems));
    dialog.push_str(&format!("add_textbox|`wLevel: `5{}``|left|\n", player.level));
    dialog.push_str("add_spacer|small|\n");
    dialog.push_str("add_button|set_online_status|Set Status|noflags|0|0|\n");
    dialog.push_str("end_dialog|profile_menu|Cancel|OK|\n");
    dialog.push_str("add_quick_exit|\n");
    dialog
}

pub fn build_farmer_menu(player: &player::Player, net_id: i32) -> String {
    let mut dialog = String::from("set_default_color|`o\n");
    dialog.push_str(&format!("embed_data|netID|{}|\n", net_id));
    dialog.push_str("add_popup_name|WrenchMenu|\n");

    let next_level_xp = crate::game::gt_mmo::get_xp_required(player.farmer_lvl as u32);


    dialog.push_str(&format!(
        "add_player_info|`2[{}]``|{}|{}|{}|\n",
        player.name,
        player.farmer_lvl,
        player.farmer_xp,
        next_level_xp
    ));

    dialog.push_str("add_spacer|small|\n");
    dialog.push_str("set_custom_spacing|x:5;y:10|\n");

    dialog.push_str("add_custom_button|goals|image:interface/large/gui_wrench_goals_quests.rttex;image_size:400,260;width:0.19;|\n");

    dialog.push_str("add_custom_break|\n");
    dialog.push_str("add_spacer|small|\n");
    dialog.push_str("set_custom_spacing|x:0;y:0|\n");

    dialog.push_str("end_dialog|popup||Continue|\n");
    dialog.push_str("add_quick_exit|\n");
    dialog
}

pub fn build_milestones_menu(player: &player::Player, net_id: i32) -> String {
    let mut dialog = String::from("set_default_color|`o\n");
    dialog.push_str(&format!("embed_data|netID|{}|\n", net_id));
    dialog.push_str("add_popup_name|milestones_menu|\n");
    dialog.push_str("add_label_with_icon|big|`wFarmer Milestones``|left|1366|\n");
    dialog.push_str("add_spacer|small|\n");

    let current_lvl = player.farmer_lvl as u32;


    let milestones = [1, 10, 25, 50, 75, 100, 150, 200];

    for &lvl in &milestones {
        let title = crate::game::gt_mmo::get_milestone_title(lvl).unwrap_or("Unknown");
        let status = if current_lvl >= lvl { "`2(Unlocked)``" } else { "`4(Locked)``" };
        dialog.push_str(&format!("add_textbox|Level {}: {} {}|left|\n", lvl, title, status));
    }

    dialog.push_str("add_spacer|small|\n");
    dialog.push_str("end_dialog|milestones|OK||\n");
    dialog.push_str("add_quick_exit|\n");
    dialog
}