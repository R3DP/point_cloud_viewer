//! A bounding box with an arbitrary 3D pose.

use crate::math::{intersects_aabb3, Cuboid, Isometry3, PointCulling};
use cgmath::{BaseFloat, EuclideanSpace, InnerSpace, Point3, Quaternion, Vector3};
use collision::{Aabb, Aabb3};
use num_traits::identities::One;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Obb<S> {
    query_from_obb: Isometry3<S>,
    obb_from_query: Isometry3<S>,
    half_extent: Vector3<S>,
    corners: [Point3<S>; 8],
    pub separating_axes: Vec<Vector3<S>>,
}

impl<S: BaseFloat> From<&Aabb3<S>> for Obb<S> {
    fn from(aabb: &Aabb3<S>) -> Self {
        Obb::new(
            Isometry3::new(Quaternion::one(), EuclideanSpace::to_vec(aabb.center())),
            aabb.dim() / (S::one() + S::one()),
        )
    }
}

impl<S: BaseFloat> From<Aabb3<S>> for Obb<S> {
    fn from(aabb: Aabb3<S>) -> Self {
        Self::from(&aabb)
    }
}

impl<S: BaseFloat> Obb<S> {
    pub fn new(query_from_obb: Isometry3<S>, half_extent: Vector3<S>) -> Self {
        Obb {
            obb_from_query: query_from_obb.inverse(),
            half_extent,
            corners: Obb::precompute_corners(&query_from_obb, &half_extent),
            separating_axes: Obb::precompute_separating_axes(&query_from_obb.rotation),
            query_from_obb,
        }
    }

    pub fn transformed(&self, global_from_query: &Isometry3<S>) -> Self {
        Self::new(global_from_query * &self.query_from_obb, self.half_extent)
    }

    fn precompute_corners(
        query_from_obb: &Isometry3<S>,
        half_extent: &Vector3<S>,
    ) -> [Point3<S>; 8] {
        let corner_from = |x, y, z| query_from_obb * &Point3::new(x, y, z);
        [
            corner_from(-half_extent.x, -half_extent.y, -half_extent.z),
            corner_from(half_extent.x, -half_extent.y, -half_extent.z),
            corner_from(-half_extent.x, half_extent.y, -half_extent.z),
            corner_from(half_extent.x, half_extent.y, -half_extent.z),
            corner_from(-half_extent.x, -half_extent.y, half_extent.z),
            corner_from(half_extent.x, -half_extent.y, half_extent.z),
            corner_from(-half_extent.x, half_extent.y, half_extent.z),
            corner_from(half_extent.x, half_extent.y, half_extent.z),
        ]
    }

    fn precompute_separating_axes(query_from_obb: &Quaternion<S>) -> Vec<Vector3<S>> {
        let unit_x = Vector3::unit_x();
        let unit_y = Vector3::unit_y();
        let unit_z = Vector3::unit_z();
        let rot_x = query_from_obb * unit_x;
        let rot_y = query_from_obb * unit_y;
        let rot_z = query_from_obb * unit_z;
        let mut separating_axes = vec![unit_x, unit_y, unit_z];
        for axis in &[
            rot_x,
            rot_y,
            rot_z,
            unit_x.cross(rot_x).normalize(),
            unit_x.cross(rot_y).normalize(),
            unit_x.cross(rot_z).normalize(),
            unit_y.cross(rot_x).normalize(),
            unit_y.cross(rot_y).normalize(),
            unit_y.cross(rot_z).normalize(),
            unit_z.cross(rot_x).normalize(),
            unit_z.cross(rot_y).normalize(),
            unit_z.cross(rot_z).normalize(),
        ] {
            let is_finite_and_non_parallel = is_finite(&axis)
                && separating_axes.iter().all(|elem| {
                    (elem - axis).magnitude() > S::default_epsilon()
                        && (elem + axis).magnitude() > S::default_epsilon()
                });
            if is_finite_and_non_parallel {
                separating_axes.push(*axis);
            }
        }
        separating_axes
    }
}

impl<S> PointCulling<S> for Obb<S>
where
    S: 'static + BaseFloat + Sync + Send,
{
    fn contains(&self, p: &Point3<S>) -> bool {
        let Point3 { x, y, z } = &self.obb_from_query * p;
        x.abs() <= self.half_extent.x
            && y.abs() <= self.half_extent.y
            && z.abs() <= self.half_extent.z
    }
    fn intersects_aabb3(&self, aabb: &Aabb3<S>) -> bool {
        intersects_aabb3(&self.corners, &self.separating_axes, aabb)
    }
}

impl<S> Cuboid<S> for Obb<S>
where
    S: BaseFloat,
{
    fn corners(&self) -> [Point3<S>; 8] {
        self.corners
    }
}

// This guards against the separating axes being NaN, which may happen when the
// orientation aligns with the unit axes.
fn is_finite<S: BaseFloat>(vec: &Vector3<S>) -> bool {
    vec.x.is_finite() && vec.y.is_finite() && vec.z.is_finite()
}
