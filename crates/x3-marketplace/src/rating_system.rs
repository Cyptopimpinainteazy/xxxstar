//! Rating System — Plugin reviews and star ratings
//!
//! Manages user reviews, ratings, and aggregated scoring for plugins

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::Result;

/// User review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    pub id: String,
    pub reviewer: String,
    pub plugin_id: String,
    pub rating: u32, // 1-5 stars
    pub title: String,
    pub content: String,
    pub helpful_count: u32,
    pub unhelpful_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub verified_user: bool,
}

impl Review {
    /// Helpfulness score (helpful - unhelpful)
    pub fn helpfulness_score(&self) -> i32 {
        self.helpful_count as i32 - self.unhelpful_count as i32
    }

    /// Is review helpful (more helpful than unhelpful)
    pub fn is_helpful(&self) -> bool {
        self.helpful_count > self.unhelpful_count
    }

    /// Days since review posted
    pub fn days_since_posted(&self) -> i64 {
        (Utc::now() - self.created_at).num_days()
    }

    /// Review age classification
    pub fn age_category(&self) -> &'static str {
        match self.days_since_posted() {
            0..=7 => "Recent",
            8..=30 => "Recent",
            31..=90 => "Moderate",
            91..=180 => "Older",
            _ => "Archived",
        }
    }
}

/// Aggregated rating statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatingStats {
    pub total_reviews: u32,
    pub average_rating: f64,
    pub rating_distribution: [u32; 5], // [1-star, 2-star, 3-star, 4-star, 5-star]
    pub helpful_reviews: u32,
    pub percent_recommended: f64,
}

impl RatingStats {
    /// Quality score (0-100 based on ratings distribution)
    pub fn quality_score(&self) -> f64 {
        if self.total_reviews == 0 {
            return 0.0;
        }
        
        let weighted_sum = 
            self.rating_distribution[0] as f64 * 1.0 +
            self.rating_distribution[1] as f64 * 2.0 +
            self.rating_distribution[2] as f64 * 3.0 +
            self.rating_distribution[3] as f64 * 4.0 +
            self.rating_distribution[4] as f64 * 5.0;

        (weighted_sum / (self.total_reviews as f64 * 5.0)) * 100.0
    }

    /// Confidence score (0-100, based on review count)
    pub fn confidence_score(&self) -> f64 {
        ((self.total_reviews as f64 / 100.0) * 100.0).min(100.0)
    }

    /// Is plugin well-rated (avg > 4.0)
    pub fn is_well_rated(&self) -> bool {
        self.average_rating >= 4.0
    }

    /// Is plugin poorly-rated (avg < 2.5)
    pub fn is_poorly_rated(&self) -> bool {
        self.average_rating < 2.5
    }
}

/// Rating System Manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatingSystem {
    reviews: HashMap<String, Review>,
    review_counter: u32,
    by_plugin: HashMap<String, Vec<String>>,
    by_reviewer: HashMap<String, Vec<String>>,
}

impl RatingSystem {
    pub fn new() -> Self {
        RatingSystem {
            reviews: HashMap::new(),
            review_counter: 0,
            by_plugin: HashMap::new(),
            by_reviewer: HashMap::new(),
        }
    }

    /// Submit review
    pub fn submit_review(
        &mut self,
        reviewer: &str,
        plugin_id: &str,
        rating: u32,
        title: String,
        content: String,
        verified_user: bool,
    ) -> Result<String> {
        if rating < 1 || rating > 5 {
            return Err(crate::MarketplaceError::InvalidRating(
                "Rating must be 1-5".to_string(),
            ));
        }

        self.review_counter += 1;
        let review_id = format!("review_{}", self.review_counter);

        let review = Review {
            id: review_id.clone(),
            reviewer: reviewer.to_string(),
            plugin_id: plugin_id.to_string(),
            rating,
            title,
            content,
            helpful_count: 0,
            unhelpful_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            verified_user,
        };

        self.reviews.insert(review_id.clone(), review);
        self.by_plugin
            .entry(plugin_id.to_string())
            .or_insert_with(Vec::new)
            .push(review_id.clone());
        self.by_reviewer
            .entry(reviewer.to_string())
            .or_insert_with(Vec::new)
            .push(review_id.clone());

        Ok(review_id)
    }

    /// Get review by ID
    pub fn get_review(&self, review_id: &str) -> Option<Review> {
        self.reviews.get(review_id).cloned()
    }

    /// Get reviews for plugin
    pub fn plugin_reviews(&self, plugin_id: &str) -> Vec<Review> {
        self.by_plugin
            .get(plugin_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.reviews.get(id))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get reviews by reviewer
    pub fn reviewer_reviews(&self, reviewer: &str) -> Vec<Review> {
        self.by_reviewer
            .get(reviewer)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.reviews.get(id))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get rating statistics for plugin
    pub fn plugin_stats(&self, plugin_id: &str) -> RatingStats {
        let reviews = self.plugin_reviews(plugin_id);

        if reviews.is_empty() {
            return RatingStats {
                total_reviews: 0,
                average_rating: 0.0,
                rating_distribution: [0; 5],
                helpful_reviews: 0,
                percent_recommended: 0.0,
            };
        }

        let mut distribution = [0u32; 5];
        let mut sum = 0u32;
        let mut helpful = 0u32;

        for review in &reviews {
            distribution[(review.rating - 1) as usize] += 1;
            sum += review.rating;
            if review.is_helpful() {
                helpful += 1;
            }
        }

        let average_rating = sum as f64 / reviews.len() as f64;
        let percent_recommended = (helpful as f64 / reviews.len() as f64) * 100.0;

        RatingStats {
            total_reviews: reviews.len() as u32,
            average_rating,
            rating_distribution: distribution,
            helpful_reviews: helpful,
            percent_recommended,
        }
    }

    /// Mark review as helpful
    pub fn mark_helpful(&mut self, review_id: &str) -> Result<()> {
        if let Some(review) = self.reviews.get_mut(review_id) {
            review.helpful_count += 1;
            review.updated_at = Utc::now();
            Ok(())
        } else {
            Err(crate::MarketplaceError::PluginNotFound)
        }
    }

    /// Mark review as unhelpful
    pub fn mark_unhelpful(&mut self, review_id: &str) -> Result<()> {
        if let Some(review) = self.reviews.get_mut(review_id) {
            review.unhelpful_count += 1;
            review.updated_at = Utc::now();
            Ok(())
        } else {
            Err(crate::MarketplaceError::PluginNotFound)
        }
    }

    /// Get top reviews for plugin (sorted by helpfulness)
    pub fn top_reviews(&self, plugin_id: &str, limit: usize) -> Vec<Review> {
        let mut reviews = self.plugin_reviews(plugin_id);
        reviews.sort_by(|a, b| {
            b.helpfulness_score()
                .cmp(&a.helpfulness_score())
                .then_with(|| b.created_at.cmp(&a.created_at))
        });
        reviews.into_iter().take(limit).collect()
    }

    /// Get recent reviews
    pub fn recent_reviews(&self, plugin_id: &str, days: i64) -> Vec<Review> {
        let cutoff = Utc::now() - chrono::Duration::days(days);
        self.plugin_reviews(plugin_id)
            .into_iter()
            .filter(|r| r.created_at > cutoff)
            .collect()
    }

    /// Delete review (by reviewer)
    pub fn delete_review(&mut self, review_id: &str, reviewer: &str) -> Result<()> {
        if let Some(review) = self.reviews.get(review_id) {
            if review.reviewer != reviewer {
                return Err(crate::MarketplaceError::PluginNotFound); // Use as permission error
            }
        }

        self.reviews.remove(review_id);
        Ok(())
    }

    /// Edit review
    pub fn edit_review(
        &mut self,
        review_id: &str,
        reviewer: &str,
        new_title: String,
        new_content: String,
    ) -> Result<()> {
        if let Some(review) = self.reviews.get_mut(review_id) {
            if review.reviewer != reviewer {
                return Err(crate::MarketplaceError::PluginNotFound); // Use as permission error
            }
            review.title = new_title;
            review.content = new_content;
            review.updated_at = Utc::now();
            Ok(())
        } else {
            Err(crate::MarketplaceError::PluginNotFound)
        }
    }

    /// Get verified user reviews for plugin
    pub fn verified_reviews(&self, plugin_id: &str) -> Vec<Review> {
        self.plugin_reviews(plugin_id)
            .into_iter()
            .filter(|r| r.verified_user)
            .collect()
    }

    /// Average rating from verified users only
    pub fn verified_average(&self, plugin_id: &str) -> f64 {
        let verified = self.verified_reviews(plugin_id);
        if verified.is_empty() {
            return 0.0;
        }
        verified.iter().map(|r| r.rating).sum::<u32>() as f64 / verified.len() as f64
    }
}

impl Default for RatingSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_submit_review() {
        let mut system = RatingSystem::new();
        let review_id = system
            .submit_review(
                "user1",
                "plugin1",
                5,
                "Great plugin!".to_string(),
                "Works well".to_string(),
                true,
            )
            .unwrap();

        assert!(!review_id.is_empty());
    }

    #[test]
    fn test_invalid_rating() {
        let mut system = RatingSystem::new();
        let result = system.submit_review(
            "user1",
            "plugin1",
            6,
            "Bad".to_string(),
            "Content".to_string(),
            true,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_plugin_stats() {
        let mut system = RatingSystem::new();
        system
            .submit_review(
                "user1",
                "plugin1",
                5,
                "Great".to_string(),
                "Content".to_string(),
                true,
            )
            .unwrap();
        system
            .submit_review(
                "user2",
                "plugin1",
                4,
                "Good".to_string(),
                "Content".to_string(),
                true,
            )
            .unwrap();

        let stats = system.plugin_stats("plugin1");
        assert_eq!(stats.total_reviews, 2);
        assert!(stats.average_rating > 4.0);
    }

    #[test]
    fn test_mark_helpful() {
        let mut system = RatingSystem::new();
        let review_id = system
            .submit_review(
                "user1",
                "plugin1",
                5,
                "Great".to_string(),
                "Content".to_string(),
                true,
            )
            .unwrap();

        system.mark_helpful(&review_id).unwrap();
        let review = system.get_review(&review_id).unwrap();
        assert_eq!(review.helpful_count, 1);
    }

    #[test]
    fn test_rating_distribution() {
        let mut system = RatingSystem::new();
        system
            .submit_review(
                "u1",
                "plugin1",
                5,
                "Great".to_string(),
                "Content".to_string(),
                true,
            )
            .unwrap();
        system
            .submit_review(
                "u2",
                "plugin1",
                5,
                "Great".to_string(),
                "Content".to_string(),
                true,
            )
            .unwrap();
        system
            .submit_review(
                "u3",
                "plugin1",
                1,
                "Bad".to_string(),
                "Content".to_string(),
                true,
            )
            .unwrap();

        let stats = system.plugin_stats("plugin1");
        assert_eq!(stats.rating_distribution[4], 2); // 5-star count
        assert_eq!(stats.rating_distribution[0], 1); // 1-star count
    }

    #[test]
    fn test_top_reviews() {
        let mut system = RatingSystem::new();
        system
            .submit_review(
                "u1",
                "plugin1",
                5,
                "Great".to_string(),
                "Content".to_string(),
                true,
            )
            .unwrap();
        system
            .submit_review(
                "u2",
                "plugin1",
                4,
                "Good".to_string(),
                "Content".to_string(),
                true,
            )
            .unwrap();

        let top = system.top_reviews("plugin1", 1);
        assert_eq!(top.len(), 1);
    }

    #[test]
    fn test_is_well_rated() {
        let stats = RatingStats {
            total_reviews: 10,
            average_rating: 4.5,
            rating_distribution: [0, 0, 0, 2, 8],
            helpful_reviews: 8,
            percent_recommended: 80.0,
        };

        assert!(stats.is_well_rated());
        assert!(!stats.is_poorly_rated());
    }

    #[test]
    fn test_verified_average() {
        let mut system = RatingSystem::new();
        system
            .submit_review(
                "u1",
                "plugin1",
                5,
                "Great".to_string(),
                "Content".to_string(),
                true,
            )
            .unwrap();
        system
            .submit_review(
                "u2",
                "plugin1",
                3,
                "Ok".to_string(),
                "Content".to_string(),
                false,
            )
            .unwrap();

        let avg = system.verified_average("plugin1");
        assert_eq!(avg, 5.0); // Only verified (5) is counted
    }

    #[test]
    fn test_edit_review() {
        let mut system = RatingSystem::new();
        let review_id = system
            .submit_review(
                "user1",
                "plugin1",
                5,
                "Great".to_string(),
                "Content".to_string(),
                true,
            )
            .unwrap();

        system
            .edit_review(
                &review_id,
                "user1",
                "Updated".to_string(),
                "New content".to_string(),
            )
            .unwrap();

        let review = system.get_review(&review_id).unwrap();
        assert_eq!(review.title, "Updated");
    }

    #[test]
    fn test_quality_score() {
        let stats = RatingStats {
            total_reviews: 100,
            average_rating: 4.0,
            rating_distribution: [0, 0, 0, 50, 50],
            helpful_reviews: 80,
            percent_recommended: 80.0,
        };

        let score = stats.quality_score();
        assert!(score >= 70.0 && score <= 95.0);
    }

    #[test]
    fn test_recent_reviews() {
        let mut system = RatingSystem::new();
        system
            .submit_review(
                "u1",
                "plugin1",
                5,
                "Recent".to_string(),
                "Content".to_string(),
                true,
            )
            .unwrap();

        let recent = system.recent_reviews("plugin1", 1);
        assert_eq!(recent.len(), 1);

        let old_cutoff = system.recent_reviews("plugin1", 9999);
        assert_eq!(old_cutoff.len(), 1);
    }
}
