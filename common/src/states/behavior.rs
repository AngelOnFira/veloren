use crate::{
    comp::{
        Beam, Body, CharacterState, ControlAction, Controller, ControllerInputs, Energy, Health,
        Inventory, LoadoutManip, Melee, Ori, PhysicsState, Pos, StateUpdate, Stats, Vel,
    },
    resources::DeltaTime,
    uid::Uid,
};
use specs::{
    hibitset,
    storage::{PairedStorage, SequentialRestriction},
    DerefFlaggedStorage, Entity, LazyUpdate,
};
use specs_idvs::IdvStorage;

pub trait CharacterBehavior {
    fn behavior(&self, data: &JoinData) -> StateUpdate;
    // Impl these to provide behavior for these inputs
    fn swap_equipped_weapons(&self, data: &JoinData) -> StateUpdate { StateUpdate::from(data) }
    fn manipulate_loadout(&self, data: &JoinData, _loadout_manip: LoadoutManip) -> StateUpdate {
        StateUpdate::from(data)
    }
    fn wield(&self, data: &JoinData) -> StateUpdate { StateUpdate::from(data) }
    fn glide_wield(&self, data: &JoinData) -> StateUpdate { StateUpdate::from(data) }
    fn unwield(&self, data: &JoinData) -> StateUpdate { StateUpdate::from(data) }
    fn sit(&self, data: &JoinData) -> StateUpdate { StateUpdate::from(data) }
    fn dance(&self, data: &JoinData) -> StateUpdate { StateUpdate::from(data) }
    fn sneak(&self, data: &JoinData) -> StateUpdate { StateUpdate::from(data) }
    fn stand(&self, data: &JoinData) -> StateUpdate { StateUpdate::from(data) }
    fn talk(&self, data: &JoinData) -> StateUpdate { StateUpdate::from(data) }
    fn handle_event(&self, data: &JoinData, event: ControlAction) -> StateUpdate {
        match event {
            ControlAction::SwapEquippedWeapons => self.swap_equipped_weapons(data),
            ControlAction::LoadoutManip(loadout_manip) => {
                self.manipulate_loadout(data, loadout_manip)
            },
            ControlAction::Wield => self.wield(data),
            ControlAction::GlideWield => self.glide_wield(data),
            ControlAction::Unwield => self.unwield(data),
            ControlAction::Sit => self.sit(data),
            ControlAction::Dance => self.dance(data),
            ControlAction::Sneak => self.sneak(data),
            ControlAction::Stand => self.stand(data),
            ControlAction::Talk => self.talk(data),
        }
    }
    // fn init(data: &JoinData) -> CharacterState;
}

/// Read-Only Data sent from Character Behavior System to behavior fn's
pub struct JoinData<'a> {
    pub entity: Entity,
    pub uid: &'a Uid,
    pub character: &'a CharacterState,
    pub pos: &'a Pos,
    pub vel: &'a Vel,
    pub ori: &'a Ori,
    pub dt: &'a DeltaTime,
    pub controller: &'a Controller,
    pub inputs: &'a ControllerInputs,
    pub health: &'a Health,
    pub energy: &'a Energy,
    pub inventory: &'a Inventory,
    pub body: &'a Body,
    pub physics: &'a PhysicsState,
    pub melee_attack: Option<&'a Melee>,
    pub updater: &'a LazyUpdate,
    pub stats: &'a Stats,
}

type RestrictedMut<'a, C> = PairedStorage<
    'a,
    'a,
    C,
    &'a mut DerefFlaggedStorage<C, IdvStorage<C>>,
    &'a hibitset::BitSet,
    SequentialRestriction,
>;

pub type JoinTuple<'a> = (
    Entity,
    &'a Uid,
    RestrictedMut<'a, CharacterState>,
    &'a mut Pos,
    &'a mut Vel,
    &'a mut Ori,
    RestrictedMut<'a, Energy>,
    RestrictedMut<'a, Inventory>,
    &'a mut Controller,
    &'a Health,
    &'a Body,
    &'a PhysicsState,
    Option<&'a Melee>,
    Option<&'a Beam>,
    &'a Stats,
);

impl<'a> JoinData<'a> {
    pub fn new(j: &'a JoinTuple<'a>, updater: &'a LazyUpdate, dt: &'a DeltaTime) -> Self {
        Self {
            entity: j.0,
            uid: j.1,
            character: j.2.get_unchecked(),
            pos: j.3,
            vel: j.4,
            ori: j.5,
            energy: j.6.get_unchecked(),
            inventory: j.7.get_unchecked(),
            controller: j.8,
            inputs: &j.8.inputs,
            health: j.9,
            body: j.10,
            physics: j.11,
            melee_attack: j.12,
            stats: j.14,
            updater,
            dt,
        }
    }
}
