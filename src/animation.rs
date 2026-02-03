use crate::audio::{AudioSystems, LP};
use bevy_app::{Plugin, PostStartup, Update};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Mutable, lifecycle::HookContext, prelude::*, world::DeferredWorld};
use std::{any::TypeId, collections::HashSet};

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_observer(active)
            .add_observer(advance)
            .add_observer(animation_target)
            .init_resource::<ScheduledKeyframes>()
            .init_resource::<ScheduledDeltas>()
            .add_systems(
                PostStartup,
                (propagate_animation_target, start_roots).chain(),
            )
            .add_systems(
                Update,
                (
                    one_shot_system.in_set(AnimationSystems::Interpolate),
                    playhead.in_set(AnimationSystems::Step),
                ),
            );

        app.configure_sets(
            Update,
            (
                AnimationSystems::Step.after(AnimationSystems::Interpolate),
                AnimationSystems::Interpolate.in_set(AudioSystems::Filters),
            ),
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub enum AnimationSystems {
    Interpolate,
    Step,
}

#[derive(Component)]
#[component(immutable)]
pub struct DeltaTime(pub f32);

#[derive(Clone, Copy, Component)]
pub struct AnimationTarget(pub Entity);

impl AnimationTarget {
    pub fn entity() -> Self {
        Self(Entity::PLACEHOLDER)
    }
}

fn animation_target(
    added: On<Add, AnimationTarget>,
    mut commands: Commands,
    targets: Query<&AnimationTarget>,
) {
    if targets.get(added.entity).unwrap().0 == Entity::PLACEHOLDER {
        commands
            .entity(added.entity)
            .insert(AnimationTarget(added.entity));
    }
}

fn propagate_animation_target(
    mut commands: Commands,
    targets: Query<(Entity, &AnimationTarget)>,
    animations: Query<&Animations>,
) {
    for (entity, target) in targets.iter() {
        for entity in animations.iter_leaves::<Animations>(entity) {
            commands.entity(entity).insert(*target);
        }
    }
}

#[derive(Component)]
#[relationship_target(relationship = AnimationOf)]
pub struct Animations(Vec<Entity>);

#[macro_export]
macro_rules! animations {
    [$($child:expr),*$(,)?] => {
        bevy_ecs::related!($crate::animation::Animations [$($child),*])
    };
}

#[macro_export]
macro_rules! parallel {
    [$($child:expr),*$(,)?] => {
        (bevy_ecs::related!($crate::animation::Animations [$($child),*]), $crate::animation::Parallel)
    };
}

#[derive(Component)]
#[relationship(relationship_target = Animations)]
pub struct AnimationOf(pub Entity);

#[derive(Component)]
pub struct Parallel;

#[derive(Component)]
#[require(Playhead)]
pub struct Duration(pub f32);

#[derive(Default, Component)]
struct Playhead(f32);

#[derive(Component)]
pub struct Finished;

fn end<T: Component<Mutability = Mutable> + Lerp + Clone>(
    added: On<Add, Finished>,
    animations: Query<(&AnimationTarget, &Keyframe<T>)>,
    mut targets: Query<&mut T>,
    mut lp_targets: Query<&mut LP<T>>,
) {
    if let Ok((target, keyframe)) = animations.get(added.entity) {
        if let Ok(mut component) = targets.get_mut(target.0) {
            *component = keyframe.0.clone();
        } else if let Ok(mut component) = lp_targets.get_mut(target.0) {
            **component = keyframe.0.clone();
        } else {
            panic!(
                "animation target does not have component {}",
                std::any::type_name::<T>()
            );
        }
    }
}

#[derive(Component)]
pub struct Active;

fn start_roots(
    mut commands: Commands,
    roots: Query<Entity, (With<Animations>, Without<AnimationOf>)>,
) {
    for root in roots.iter() {
        commands.entity(root).insert(Active);
    }
}

fn active(
    inserted: On<Insert, Active>,
    mut commands: Commands,
    roots: Query<(&Animations, Option<&Playhead>, Has<Parallel>)>,
    leaves: Query<Entity, Without<Finished>>,
) {
    if let Ok((children, remainder, is_parallel)) = roots.get(inserted.entity) {
        for leaf in
            leaves
                .iter_many(children.iter())
                .take(if is_parallel { children.len() } else { 1 })
        {
            commands.entity(leaf).insert(Active);
            if let Some(remainder) = remainder {
                commands.entity(leaf).insert(Playhead(remainder.0));
            }
        }
    }
}

#[derive(Component)]
struct Advance(f32);

fn advance(
    inserted: On<Insert, Advance>,
    mut commands: Commands,
    advance: Query<&Advance>,
    roots: Query<(&Animations, Option<&AnimationOf>, Has<Parallel>)>,
    leaves: Query<Entity, Without<Finished>>,
) {
    let remainder = advance.get(inserted.entity).unwrap().0;
    let (children, parent, is_parallel) = roots.get(inserted.entity).unwrap();
    if is_parallel {
        if leaves.iter_many(children.iter()).next().is_none() {
            commands.entity(inserted.entity).insert(Finished);
            if let Some(parent) = parent {
                commands.entity(parent.0).insert(Advance(remainder));
            }
        }
    } else {
        match leaves.iter_many(children.iter()).fetch_next() {
            Some(next) => {
                commands.entity(next).insert((Active, Playhead(remainder)));
            }
            None => {
                commands.entity(inserted.entity).insert(Finished);
                if let Some(parent) = parent {
                    commands.entity(parent.0).insert(Advance(remainder));
                }
            }
        }
    }
}

fn playhead(
    mut commands: Commands,
    mut leaves: Query<(Entity, &mut Playhead, &Duration, &AnimationOf), With<Active>>,
    dt: Single<&DeltaTime>,
) {
    for (entity, mut playhead, duration, parent) in leaves.iter_mut() {
        playhead.0 += dt.0;
        if playhead.0 >= duration.0 {
            commands.entity(entity).remove::<Active>().insert(Finished);
            commands
                .entity(parent.0)
                .insert(Advance(playhead.0 - duration.0));
        }
    }
}

// TODO: this whole system is a mess
#[derive(Component)]
pub struct System(Option<Box<dyn FnMut(&mut World) + Send + Sync>>);

pub fn system<S, M>(s: S) -> System
where
    S: IntoSystem<(), (), M> + Send + Sync + 'static,
{
    let mut s = Some(s);
    let mut id = None;
    System(Some(Box::new(move |world: &mut World| match id {
        Some(id) => {
            world.run_system(id).unwrap();
        }
        None => {
            let system = world.register_system(s.take().unwrap());
            world.run_system(system).unwrap();
            id = Some(system);
        }
    })))
}

fn one_shot_system(
    mut commands: Commands,
    animations: Query<Entity, (With<System>, With<Active>, With<Duration>)>,
) {
    for entity in animations.iter() {
        commands.queue(move |world: &mut World| {
            let mut system = world.get_mut::<System>(entity).unwrap();
            let mut s = system.0.take().unwrap();
            s(world);
            let mut system = world.get_mut::<System>(entity).unwrap();
            _ = system.0.insert(s);
        });
    }
}

#[derive(Component)]
#[component(on_insert = schedule_keyframe::<T>)]
pub struct Keyframe<T: Component<Mutability = Mutable> + Lerp + Clone>(pub T);

#[derive(Default, Resource, Deref, DerefMut)]
struct ScheduledKeyframes(HashSet<TypeId>);

fn schedule_keyframe<T: Component<Mutability = Mutable> + Lerp + Clone>(
    mut world: DeferredWorld,
    _: HookContext,
) {
    world.commands().queue(|world: &mut World| {
        if world
            .resource_mut::<ScheduledKeyframes>()
            .insert(TypeId::of::<T>())
        {
            world.add_observer(start::<T>);
            world.add_observer(end::<T>);
            world.schedule_scope(Update, |_, schedule| {
                schedule.add_systems(keyframe::<T>.in_set(AnimationSystems::Interpolate));
            });
        }
    });
}

fn keyframe<T: Component<Mutability = Mutable> + Lerp + Clone>(
    mut targets: Query<&mut T>,
    mut lp_targets: Query<&mut LP<T>>,
    animations: Query<
        (
            &AnimationTarget,
            &Start<T>,
            &Keyframe<T>,
            &Playhead,
            &Duration,
            Option<&EaseFunction>,
        ),
        With<Active>,
    >,
) {
    for (target, start, end, playhead, duration, ease) in animations.iter() {
        if let Ok(mut component) = targets.get_mut(target.0) {
            let mut t = (playhead.0 / duration.0).clamp(0.0, 1.0);
            if let Some(ease) = ease {
                t = ease.eval(t);
            }
            *component = start.0.lerp(&end.0, t);
        } else if let Ok(mut component) = lp_targets.get_mut(target.0) {
            let mut t = (playhead.0 / duration.0).clamp(0.0, 1.0);
            if let Some(ease) = ease {
                t = ease.eval(t);
            }
            **component = start.0.lerp(&end.0, t);
        } else {
            panic!(
                "animation target does not have component {}",
                std::any::type_name::<T>()
            );
        }
    }
}

#[derive(Component)]
#[component(on_insert = schedule_delta::<T>)]
pub struct Delta<T: Component<Mutability = Mutable> + std::ops::Add<Output = T> + Lerp + Clone>(
    pub T,
);

#[derive(Default, Resource, Deref, DerefMut)]
struct ScheduledDeltas(HashSet<TypeId>);

fn schedule_delta<T: Component<Mutability = Mutable> + std::ops::Add<Output = T> + Lerp + Clone>(
    mut world: DeferredWorld,
    _: HookContext,
) {
    world.commands().queue(|world: &mut World| {
        if world
            .resource_mut::<ScheduledDeltas>()
            .insert(TypeId::of::<T>())
        {
            // TODO: doubling start here with keyframes
            world.add_observer(start::<T>);
            world.schedule_scope(Update, |_, schedule| {
                schedule.add_systems(delta::<T>.in_set(AnimationSystems::Interpolate));
            });
        }
    });
}

fn delta<T: Component<Mutability = Mutable> + std::ops::Add<Output = T> + Lerp + Clone>(
    mut targets: Query<&mut T>,
    mut lp_targets: Query<&mut LP<T>>,
    animations: Query<
        (
            &AnimationTarget,
            &Start<T>,
            &Delta<T>,
            &Playhead,
            &Duration,
            Option<&EaseFunction>,
        ),
        With<Active>,
    >,
) {
    for (target, start, delta, playhead, duration, ease) in animations.iter() {
        if let Ok(mut component) = targets.get_mut(target.0) {
            let mut t = (playhead.0 / duration.0).clamp(0.0, 1.0);
            if let Some(ease) = ease {
                t = ease.eval(t);
            }
            let end = delta.0.clone().add(start.0.clone());
            *component = start.0.lerp(&end, t);
        } else if let Ok(mut component) = lp_targets.get_mut(target.0) {
            let mut t = (playhead.0 / duration.0).clamp(0.0, 1.0);
            if let Some(ease) = ease {
                t = ease.eval(t);
            }
            let end = delta.0.clone().add(start.0.clone());
            **component = start.0.lerp(&end, t);
        } else {
            panic!(
                "animation target does not have component {}",
                std::any::type_name::<T>()
            );
        }
    }
}

#[derive(Component)]
struct Start<T: Component>(T);

fn start<T: Component + Clone>(
    inserted: On<Add, Active>,
    mut commands: Commands,
    targets: Query<&AnimationTarget>,
    components: Query<&T>,
    lp_components: Query<&LP<T>>,
) {
    if let Ok(target) = targets.get(inserted.entity) {
        match components.get(target.0) {
            Ok(current) => {
                commands
                    .entity(inserted.entity)
                    .insert(Start(current.clone()));
            }
            Err(_) => match lp_components.get(target.0) {
                Ok(current) => {
                    commands
                        .entity(inserted.entity)
                        .insert(Start((*current).clone()));
                }
                Err(_) => {
                    panic!(
                        "animation target does not have component {}",
                        std::any::type_name::<T>()
                    );
                }
            },
        }
    }
}

pub trait Lerp {
    fn lerp(&self, rhs: &Self, t: f32) -> Self;
}

impl Lerp for f32 {
    fn lerp(&self, rhs: &Self, t: f32) -> Self {
        *self * (1.0 - t) + *rhs * t
    }
}

impl Lerp for f64 {
    fn lerp(&self, rhs: &Self, t: f32) -> Self {
        *self * (1.0 - t as f64) + *rhs * t as f64
    }
}

#[derive(Clone, Copy)]
pub struct LogF32(pub f32);

impl Lerp for LogF32 {
    fn lerp(&self, rhs: &Self, t: f32) -> Self {
        Self((self.0.ln() * (1.0 - t) + rhs.0.ln() * t).exp())
    }
}

macro_rules! lerp_prim {
    ($prim:ident) => {
        impl Lerp for $prim {
            fn lerp(&self, rhs: &Self, t: f32) -> Self {
                (*self as f32).lerp(&(*rhs as f32), t).round() as $prim
            }
        }
    };
}

lerp_prim!(u8);
lerp_prim!(u16);
lerp_prim!(u32);
lerp_prim!(u64);
lerp_prim!(usize);

lerp_prim!(i8);
lerp_prim!(i16);
lerp_prim!(i32);
lerp_prim!(i64);
lerp_prim!(isize);

#[macro_export]
macro_rules! lerp_newtype {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident($ivis:vis $type:ty);
    ) => {
        $(#[$meta])*
        $vis struct $name($ivis $type);
        impl $crate::animation::Lerp for $name {
            fn lerp(&self, rhs: &Self, t: f32) -> Self {
                $name(self.0.lerp(&rhs.0, t))
            }
        }
        impl std::ops::Add for $name {
            type Output = $name;
            fn add(self, rhs: Self) -> Self::Output {
                $name(self.0 + rhs.0)
            }
        }
    };
}

#[derive(Clone, Copy, Component)]
pub enum EaseFunction {
    /// `f(t) = t`
    Linear,

    /// `f(t) = t²`
    ///
    /// This is the Hermite interpolator for
    /// - f(0) = 0
    /// - f(1) = 1
    /// - f′(0) = 0
    QuadraticIn,
    /// `f(t) = -(t * (t - 2.0))`
    ///
    /// This is the Hermite interpolator for
    /// - f(0) = 0
    /// - f(1) = 1
    /// - f′(1) = 0
    QuadraticOut,
    /// Behaves as `EaseFunction::QuadraticIn` for t < 0.5 and as `EaseFunction::QuadraticOut` for t >= 0.5
    ///
    /// A quadratic has too low of a degree to be both an `InOut` and C²,
    /// so consider using at least a cubic (such as [`EaseFunction::SmoothStep`])
    /// if you want the acceleration to be continuous.
    QuadraticInOut,

    /// `f(t) = t³`
    ///
    /// This is the Hermite interpolator for
    /// - f(0) = 0
    /// - f(1) = 1
    /// - f′(0) = 0
    /// - f″(0) = 0
    CubicIn,
    /// `f(t) = (t - 1.0)³ + 1.0`
    CubicOut,
    /// Behaves as `EaseFunction::CubicIn` for t < 0.5 and as `EaseFunction::CubicOut` for t >= 0.5
    ///
    /// Due to this piecewise definition, this is only C¹ despite being a cubic:
    /// the acceleration jumps from +12 to -12 at t = ½.
    ///
    /// Consider using [`EaseFunction::SmoothStep`] instead, which is also cubic,
    /// or [`EaseFunction::SmootherStep`] if you picked this because you wanted
    /// the acceleration at the endpoints to also be zero.
    CubicInOut,

    /// `f(t) = t⁴`
    QuarticIn,
    /// `f(t) = (t - 1.0)³ * (1.0 - t) + 1.0`
    QuarticOut,
    /// Behaves as `EaseFunction::QuarticIn` for t < 0.5 and as `EaseFunction::QuarticOut` for t >= 0.5
    QuarticInOut,

    /// `f(t) = t⁵`
    QuinticIn,
    /// `f(t) = (t - 1.0)⁵ + 1.0`
    QuinticOut,
    /// Behaves as `EaseFunction::QuinticIn` for t < 0.5 and as `EaseFunction::QuinticOut` for t >= 0.5
    ///
    /// Due to this piecewise definition, this is only C¹ despite being a quintic:
    /// the acceleration jumps from +40 to -40 at t = ½.
    ///
    /// Consider using [`EaseFunction::SmootherStep`] instead, which is also quintic.
    QuinticInOut,

    /// Behaves as the first half of [`EaseFunction::SmoothStep`].
    ///
    /// This has f″(1) = 0, unlike [`EaseFunction::QuadraticIn`] which starts similarly.
    SmoothStepIn,
    /// Behaves as the second half of [`EaseFunction::SmoothStep`].
    ///
    /// This has f″(0) = 0, unlike [`EaseFunction::QuadraticOut`] which ends similarly.
    SmoothStepOut,
    /// `f(t) = 3t² - 2t³`
    ///
    /// This is the Hermite interpolator for
    /// - f(0) = 0
    /// - f(1) = 1
    /// - f′(0) = 0
    /// - f′(1) = 0
    ///
    /// See also [`smoothstep` in GLSL][glss].
    ///
    /// [glss]: https://registry.khronos.org/OpenGL-Refpages/gl4/html/smoothstep.xhtml
    SmoothStep,

    /// Behaves as the first half of [`EaseFunction::SmootherStep`].
    ///
    /// This has f″(1) = 0, unlike [`EaseFunction::CubicIn`] which starts similarly.
    SmootherStepIn,
    /// Behaves as the second half of [`EaseFunction::SmootherStep`].
    ///
    /// This has f″(0) = 0, unlike [`EaseFunction::CubicOut`] which ends similarly.
    SmootherStepOut,
    /// `f(t) = 6t⁵ - 15t⁴ + 10t³`
    ///
    /// This is the Hermite interpolator for
    /// - f(0) = 0
    /// - f(1) = 1
    /// - f′(0) = 0
    /// - f′(1) = 0
    /// - f″(0) = 0
    /// - f″(1) = 0
    SmootherStep,

    /// `f(t) = 1.0 - cos(t * π / 2.0)`
    SineIn,
    /// `f(t) = sin(t * π / 2.0)`
    SineOut,
    /// Behaves as `EaseFunction::SineIn` for t < 0.5 and as `EaseFunction::SineOut` for t >= 0.5
    SineInOut,

    /// `f(t) = 1.0 - sqrt(1.0 - t²)`
    CircularIn,
    /// `f(t) = sqrt((2.0 - t) * t)`
    CircularOut,
    /// Behaves as `EaseFunction::CircularIn` for t < 0.5 and as `EaseFunction::CircularOut` for t >= 0.5
    CircularInOut,

    /// `f(t) ≈ 2.0^(10.0 * (t - 1.0))`
    ///
    /// The precise definition adjusts it slightly so it hits both `(0, 0)` and `(1, 1)`:
    /// `f(t) = 2.0^(10.0 * t - A) - B`, where A = log₂(2¹⁰-1) and B = 1/(2¹⁰-1).
    ExponentialIn,
    /// `f(t) ≈ 1.0 - 2.0^(-10.0 * t)`
    ///
    /// As with `EaseFunction::ExponentialIn`, the precise definition adjusts it slightly
    // so it hits both `(0, 0)` and `(1, 1)`.
    ExponentialOut,
    /// Behaves as `EaseFunction::ExponentialIn` for t < 0.5 and as `EaseFunction::ExponentialOut` for t >= 0.5
    ExponentialInOut,

    /// `f(t) = -2.0^(10.0 * t - 10.0) * sin((t * 10.0 - 10.75) * 2.0 * π / 3.0)`
    ElasticIn,
    /// `f(t) = 2.0^(-10.0 * t) * sin((t * 10.0 - 0.75) * 2.0 * π / 3.0) + 1.0`
    ElasticOut,
    /// Behaves as `EaseFunction::ElasticIn` for t < 0.5 and as `EaseFunction::ElasticOut` for t >= 0.5
    ElasticInOut,

    /// `f(t) = 2.70158 * t³ - 1.70158 * t²`
    BackIn,
    /// `f(t) = 1.0 +  2.70158 * (t - 1.0)³ - 1.70158 * (t - 1.0)²`
    BackOut,
    /// Behaves as `EaseFunction::BackIn` for t < 0.5 and as `EaseFunction::BackOut` for t >= 0.5
    BackInOut,

    /// bouncy at the start!
    BounceIn,
    /// bouncy at the end!
    BounceOut,
    /// Behaves as `EaseFunction::BounceIn` for t < 0.5 and as `EaseFunction::BounceOut` for t >= 0.5
    BounceInOut,
}

trait Squared {
    fn squared(self) -> Self;
}

impl Squared for f32 {
    fn squared(self) -> Self {
        self * self
    }
}

trait Cubed {
    fn cubed(self) -> Self;
}

impl Cubed for f32 {
    fn cubed(self) -> Self {
        self * self * self
    }
}

mod easing_functions {
    use super::{Cubed, Squared};
    use core::f32::consts::{FRAC_PI_2, FRAC_PI_3, PI};

    mod ops {
        pub fn sin(x: f32) -> f32 {
            x.sin()
        }
        pub fn cos(x: f32) -> f32 {
            x.cos()
        }
        pub fn sqrt(x: f32) -> f32 {
            x.sqrt()
        }
        pub fn powf(x: f32, f: f32) -> f32 {
            x.powf(f)
        }
        pub fn exp2(x: f32) -> f32 {
            x.exp2()
        }
    }

    #[inline]
    pub(crate) fn linear(t: f32) -> f32 {
        t
    }

    #[inline]
    pub(crate) fn quadratic_in(t: f32) -> f32 {
        t.squared()
    }
    #[inline]
    pub(crate) fn quadratic_out(t: f32) -> f32 {
        1.0 - (1.0 - t).squared()
    }
    #[inline]
    pub(crate) fn quadratic_in_out(t: f32) -> f32 {
        if t < 0.5 {
            2.0 * t.squared()
        } else {
            1.0 - (-2.0 * t + 2.0).squared() / 2.0
        }
    }

    #[inline]
    pub(crate) fn cubic_in(t: f32) -> f32 {
        t.cubed()
    }
    #[inline]
    pub(crate) fn cubic_out(t: f32) -> f32 {
        1.0 - (1.0 - t).cubed()
    }
    #[inline]
    pub(crate) fn cubic_in_out(t: f32) -> f32 {
        if t < 0.5 {
            4.0 * t.cubed()
        } else {
            1.0 - (-2.0 * t + 2.0).cubed() / 2.0
        }
    }

    #[inline]
    pub(crate) fn quartic_in(t: f32) -> f32 {
        t * t * t * t
    }
    #[inline]
    pub(crate) fn quartic_out(t: f32) -> f32 {
        1.0 - (1.0 - t) * (1.0 - t) * (1.0 - t) * (1.0 - t)
    }
    #[inline]
    pub(crate) fn quartic_in_out(t: f32) -> f32 {
        if t < 0.5 {
            8.0 * t * t * t * t
        } else {
            1.0 - (-2.0 * t + 2.0) * (-2.0 * t + 2.0) * (-2.0 * t + 2.0) * (-2.0 * t + 2.0) / 2.0
        }
    }

    #[inline]
    pub(crate) fn quintic_in(t: f32) -> f32 {
        t * t * t * t * t
    }
    #[inline]
    pub(crate) fn quintic_out(t: f32) -> f32 {
        1.0 - (1.0 - t) * (1.0 - t) * (1.0 - t) * (1.0 - t) * (1.0 - t)
    }
    #[inline]
    pub(crate) fn quintic_in_out(t: f32) -> f32 {
        if t < 0.5 {
            16.0 * t * t * t * t * t
        } else {
            1.0 - (-2.0 * t + 2.0)
                * (-2.0 * t + 2.0)
                * (-2.0 * t + 2.0)
                * (-2.0 * t + 2.0)
                * (-2.0 * t + 2.0)
                / 2.0
        }
    }

    #[inline]
    pub(crate) fn smoothstep_in(t: f32) -> f32 {
        ((1.5 - 0.5 * t) * t) * t
    }

    #[inline]
    pub(crate) fn smoothstep_out(t: f32) -> f32 {
        (1.5 + (-0.5 * t) * t) * t
    }

    #[inline]
    pub(crate) fn smoothstep(t: f32) -> f32 {
        ((3.0 - 2.0 * t) * t) * t
    }

    #[inline]
    pub(crate) fn smootherstep_in(t: f32) -> f32 {
        (((2.5 + (-1.875 + 0.375 * t) * t) * t) * t) * t
    }

    #[inline]
    pub(crate) fn smootherstep_out(t: f32) -> f32 {
        (1.875 + ((-1.25 + (0.375 * t) * t) * t) * t) * t
    }

    #[inline]
    pub(crate) fn smootherstep(t: f32) -> f32 {
        (((10.0 + (-15.0 + 6.0 * t) * t) * t) * t) * t
    }

    #[inline]
    pub(crate) fn sine_in(t: f32) -> f32 {
        1.0 - ops::cos(t * FRAC_PI_2)
    }
    #[inline]
    pub(crate) fn sine_out(t: f32) -> f32 {
        ops::sin(t * FRAC_PI_2)
    }
    #[inline]
    pub(crate) fn sine_in_out(t: f32) -> f32 {
        -(ops::cos(PI * t) - 1.0) / 2.0
    }

    #[inline]
    pub(crate) fn circular_in(t: f32) -> f32 {
        1.0 - ops::sqrt(1.0 - t.squared())
    }
    #[inline]
    pub(crate) fn circular_out(t: f32) -> f32 {
        ops::sqrt(1.0 - (t - 1.0).squared())
    }
    #[inline]
    pub(crate) fn circular_in_out(t: f32) -> f32 {
        if t < 0.5 {
            (1.0 - ops::sqrt(1.0 - (2.0 * t).squared())) / 2.0
        } else {
            (ops::sqrt(1.0 - (-2.0 * t + 2.0).squared()) + 1.0) / 2.0
        }
    }

    // These are copied from a high precision calculator; I'd rather show them
    // with blatantly more digits than needed (since rust will round them to the
    // nearest representable value anyway) rather than make it seem like the
    // truncated value is somehow carefully chosen.
    #[expect(
        clippy::excessive_precision,
        reason = "This is deliberately more precise than an f32 will allow, as truncating the value might imply that the value is carefully chosen."
    )]
    const LOG2_1023: f32 = 9.998590429745328646459226;
    #[expect(
        clippy::excessive_precision,
        reason = "This is deliberately more precise than an f32 will allow, as truncating the value might imply that the value is carefully chosen."
    )]
    const FRAC_1_1023: f32 = 0.00097751710654936461388074291;
    #[inline]
    pub(crate) fn exponential_in(t: f32) -> f32 {
        // Derived from a rescaled exponential formula `(2^(10*t) - 1) / (2^10 - 1)`
        // See <https://www.wolframalpha.com/input?i=solve+over+the+reals%3A+pow%282%2C+10-A%29+-+pow%282%2C+-A%29%3D+1>
        ops::exp2(10.0 * t - LOG2_1023) - FRAC_1_1023
    }
    #[inline]
    pub(crate) fn exponential_out(t: f32) -> f32 {
        (FRAC_1_1023 + 1.0) - ops::exp2(-10.0 * t - (LOG2_1023 - 10.0))
    }
    #[inline]
    pub(crate) fn exponential_in_out(t: f32) -> f32 {
        if t < 0.5 {
            ops::exp2(20.0 * t - (LOG2_1023 + 1.0)) - (FRAC_1_1023 / 2.0)
        } else {
            (FRAC_1_1023 / 2.0 + 1.0) - ops::exp2(-20.0 * t - (LOG2_1023 - 19.0))
        }
    }

    #[inline]
    pub(crate) fn back_in(t: f32) -> f32 {
        let c = 1.70158;

        (c + 1.0) * t.cubed() - c * t.squared()
    }
    #[inline]
    pub(crate) fn back_out(t: f32) -> f32 {
        let c = 1.70158;

        1.0 + (c + 1.0) * (t - 1.0).cubed() + c * (t - 1.0).squared()
    }
    #[inline]
    pub(crate) fn back_in_out(t: f32) -> f32 {
        let c1 = 1.70158;
        let c2 = c1 + 1.525;

        if t < 0.5 {
            (2.0 * t).squared() * ((c2 + 1.0) * 2.0 * t - c2) / 2.0
        } else {
            ((2.0 * t - 2.0).squared() * ((c2 + 1.0) * (2.0 * t - 2.0) + c2) + 2.0) / 2.0
        }
    }

    #[inline]
    pub(crate) fn elastic_in(t: f32) -> f32 {
        -ops::powf(2.0, 10.0 * t - 10.0) * ops::sin((t * 10.0 - 10.75) * 2.0 * FRAC_PI_3)
    }
    #[inline]
    pub(crate) fn elastic_out(t: f32) -> f32 {
        ops::powf(2.0, -10.0 * t) * ops::sin((t * 10.0 - 0.75) * 2.0 * FRAC_PI_3) + 1.0
    }
    #[inline]
    pub(crate) fn elastic_in_out(t: f32) -> f32 {
        let c = (2.0 * PI) / 4.5;

        if t < 0.5 {
            -ops::powf(2.0, 20.0 * t - 10.0) * ops::sin((t * 20.0 - 11.125) * c) / 2.0
        } else {
            ops::powf(2.0, -20.0 * t + 10.0) * ops::sin((t * 20.0 - 11.125) * c) / 2.0 + 1.0
        }
    }

    #[inline]
    pub(crate) fn bounce_in(t: f32) -> f32 {
        1.0 - bounce_out(1.0 - t)
    }
    #[inline]
    pub(crate) fn bounce_out(t: f32) -> f32 {
        if t < 4.0 / 11.0 {
            (121.0 * t.squared()) / 16.0
        } else if t < 8.0 / 11.0 {
            (363.0 / 40.0 * t.squared()) - (99.0 / 10.0 * t) + 17.0 / 5.0
        } else if t < 9.0 / 10.0 {
            (4356.0 / 361.0 * t.squared()) - (35442.0 / 1805.0 * t) + 16061.0 / 1805.0
        } else {
            (54.0 / 5.0 * t.squared()) - (513.0 / 25.0 * t) + 268.0 / 25.0
        }
    }
    #[inline]
    pub(crate) fn bounce_in_out(t: f32) -> f32 {
        if t < 0.5 {
            (1.0 - bounce_out(1.0 - 2.0 * t)) / 2.0
        } else {
            (1.0 + bounce_out(2.0 * t - 1.0)) / 2.0
        }
    }
}

impl EaseFunction {
    pub fn eval(&self, t: f32) -> f32 {
        match self {
            EaseFunction::Linear => easing_functions::linear(t),
            EaseFunction::QuadraticIn => easing_functions::quadratic_in(t),
            EaseFunction::QuadraticOut => easing_functions::quadratic_out(t),
            EaseFunction::QuadraticInOut => easing_functions::quadratic_in_out(t),
            EaseFunction::CubicIn => easing_functions::cubic_in(t),
            EaseFunction::CubicOut => easing_functions::cubic_out(t),
            EaseFunction::CubicInOut => easing_functions::cubic_in_out(t),
            EaseFunction::QuarticIn => easing_functions::quartic_in(t),
            EaseFunction::QuarticOut => easing_functions::quartic_out(t),
            EaseFunction::QuarticInOut => easing_functions::quartic_in_out(t),
            EaseFunction::QuinticIn => easing_functions::quintic_in(t),
            EaseFunction::QuinticOut => easing_functions::quintic_out(t),
            EaseFunction::QuinticInOut => easing_functions::quintic_in_out(t),
            EaseFunction::SmoothStepIn => easing_functions::smoothstep_in(t),
            EaseFunction::SmoothStepOut => easing_functions::smoothstep_out(t),
            EaseFunction::SmoothStep => easing_functions::smoothstep(t),
            EaseFunction::SmootherStepIn => easing_functions::smootherstep_in(t),
            EaseFunction::SmootherStepOut => easing_functions::smootherstep_out(t),
            EaseFunction::SmootherStep => easing_functions::smootherstep(t),
            EaseFunction::SineIn => easing_functions::sine_in(t),
            EaseFunction::SineOut => easing_functions::sine_out(t),
            EaseFunction::SineInOut => easing_functions::sine_in_out(t),
            EaseFunction::CircularIn => easing_functions::circular_in(t),
            EaseFunction::CircularOut => easing_functions::circular_out(t),
            EaseFunction::CircularInOut => easing_functions::circular_in_out(t),
            EaseFunction::ExponentialIn => easing_functions::exponential_in(t),
            EaseFunction::ExponentialOut => easing_functions::exponential_out(t),
            EaseFunction::ExponentialInOut => easing_functions::exponential_in_out(t),
            EaseFunction::ElasticIn => easing_functions::elastic_in(t),
            EaseFunction::ElasticOut => easing_functions::elastic_out(t),
            EaseFunction::ElasticInOut => easing_functions::elastic_in_out(t),
            EaseFunction::BackIn => easing_functions::back_in(t),
            EaseFunction::BackOut => easing_functions::back_out(t),
            EaseFunction::BackInOut => easing_functions::back_in_out(t),
            EaseFunction::BounceIn => easing_functions::bounce_in(t),
            EaseFunction::BounceOut => easing_functions::bounce_out(t),
            EaseFunction::BounceInOut => easing_functions::bounce_in_out(t),
        }
    }
}
