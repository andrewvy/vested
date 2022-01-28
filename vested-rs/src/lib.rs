use chrono::{Date, Datelike, Utc};
use chronoutil::{DateRule, RelativeDuration};

#[derive(Debug, PartialEq, PartialOrd)]
struct VestingPeriod {
    date: Date<Utc>,
    cumulative_vested_amount: i32,
}

struct VestingSchedule {
    from_date: Date<Utc>,
    to_date: Date<Utc>,
    periods: Vec<VestingPeriod>,
}

enum VestingInterval {
    Monthly,
}

struct VestingScheduleConfiguration {
    interval: VestingInterval,
    cliff_percentage: f32,
    cliff: i32,
    length: i32,
}

struct Grant {
    amount: i32,
    grant_date: Date<Utc>,
    vesting_schedule: VestingScheduleConfiguration,
}

impl Grant {
    /// Calculates the difference of months between the grant date and the given future date.
    fn months_difference(&self, future_date: Date<Utc>) -> i32 {
        let year_difference = future_date.year() - self.grant_date.year();
        let months_difference =
            (year_difference * 12) + (future_date.month() as i32 - self.grant_date.month() as i32);

        return months_difference;
    }

    /// Checks if the given future date is still in the cliff period.
    fn is_before_cliff(&self, future_date: Date<Utc>) -> bool {
        return self.months_difference(future_date) < self.vesting_schedule.cliff as i32;
    }

    /// Returns the amount of vested equity when cliff period has been reached.
    fn cliff_vested_amount(&self) -> f32 {
        return self.amount as f32 * self.vesting_schedule.cliff_percentage;
    }

    /// Calculates the vested amount on a given future date.
    pub fn calculate_vested_amount(&self, future_date: Date<Utc>) -> f32 {
        match self.vesting_schedule.interval {
            VestingInterval::Monthly => {
                if self.is_before_cliff(future_date) {
                    return 0.0;
                } else if self.months_difference(future_date) > self.vesting_schedule.length {
                    return self.amount as f32;
                } else {
                    let months_past_cliff =
                        self.months_difference(future_date) - self.vesting_schedule.cliff;

                    if months_past_cliff == 0 {
                        return self.cliff_vested_amount();
                    }

                    let remaining_amount_after_cliff: f32 =
                        (self.amount as f32 - self.cliff_vested_amount()).into();
                    let vested_per_month: f32 = remaining_amount_after_cliff
                        / (self.vesting_schedule.length - self.vesting_schedule.cliff) as f32;
                    let vested_after_cliff: f32 = vested_per_month * months_past_cliff as f32;

                    return self.cliff_vested_amount() + vested_after_cliff;
                }
            }
        }
    }

    /// Calculate a full vesting schedule, listing the vested amounts per vesting period.
    pub fn calculate_vesting_schedule(&self) -> VestingSchedule {
        let duration = RelativeDuration::months(self.vesting_schedule.length);
        let to_date = self.grant_date + duration;
        let rule = DateRule::monthly(self.grant_date)
            .with_count(self.vesting_schedule.length as usize + 1);
        let periods = rule
            .map(|month| VestingPeriod {
                date: month,
                cumulative_vested_amount: self.calculate_vested_amount(month).floor() as i32,
            })
            .collect();

        return VestingSchedule {
            periods,
            from_date: self.grant_date,
            to_date,
        };
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use chrono::TimeZone;

    use crate::VestingPeriod;

    use super::{Grant, Utc, VestingInterval, VestingScheduleConfiguration};

    #[test]
    fn it_can_calculate_vested_amounts_for_given_dates() {
        let grant = Grant {
            amount: 10_000,
            grant_date: Utc.ymd(2020, 2, 6),
            vesting_schedule: VestingScheduleConfiguration {
                interval: VestingInterval::Monthly,
                cliff: 12,
                cliff_percentage: 0.25,
                length: 48,
            },
        };

        /*
         * 10,000 stock options, 25% cliff after 12 months, 48 month vesting schedule.
         * - 2021/2/6: 25% options vest, 2500 options vested
         * - 2021/3/6: +208 options vest, 2708 options vested
         * - 2022/2/6: 50% options vested, 5000 options vested
         */

        assert_relative_eq!(
            grant.calculate_vested_amount(Utc.ymd(2020, 8, 6)),
            0.0,
            max_relative = 0.005
        );
        assert_relative_eq!(
            grant.calculate_vested_amount(Utc.ymd(2020, 12, 6)),
            0.0,
            max_relative = 0.005
        );
        assert_relative_eq!(
            grant.calculate_vested_amount(Utc.ymd(2021, 2, 6)),
            2500.0,
            max_relative = 0.005
        );
        assert_relative_eq!(
            grant.calculate_vested_amount(Utc.ymd(2021, 3, 6)),
            2708.33,
            max_relative = 0.005
        );
        assert_relative_eq!(
            grant.calculate_vested_amount(Utc.ymd(2022, 2, 6)),
            5000.0,
            max_relative = 0.005
        );
        assert_relative_eq!(
            grant.calculate_vested_amount(Utc.ymd(2022, 3, 6)),
            5208.33,
            max_relative = 0.005
        );
        assert_relative_eq!(
            grant.calculate_vested_amount(Utc.ymd(2022, 4, 6)),
            5416.66,
            max_relative = 0.005
        );
        assert_relative_eq!(
            grant.calculate_vested_amount(Utc.ymd(2023, 2, 6)),
            7500.00,
            max_relative = 0.005
        );
        assert_relative_eq!(
            grant.calculate_vested_amount(Utc.ymd(2023, 3, 6)),
            7708.33,
            max_relative = 0.005
        );
        assert_relative_eq!(
            grant.calculate_vested_amount(Utc.ymd(2024, 3, 6)),
            10000.00,
            max_relative = 0.005
        );
    }

    #[test]
    fn it_can_calculate_full_vesting_schedule() {
        let grant = Grant {
            amount: 10_000,
            grant_date: Utc.ymd(2020, 2, 6),
            vesting_schedule: VestingScheduleConfiguration {
                interval: VestingInterval::Monthly,
                cliff: 6,
                cliff_percentage: 0.25,
                length: 12,
            },
        };

        let vesting_schedule = grant.calculate_vesting_schedule();

        assert_eq!(vesting_schedule.from_date, grant.grant_date);
        assert_eq!(vesting_schedule.to_date, Utc.ymd(2021, 2, 6));

        let periods = vec![
            VestingPeriod {
                date: Utc.ymd(2020, 2, 6),
                cumulative_vested_amount: 0,
            },
            VestingPeriod {
                date: Utc.ymd(2020, 3, 6),
                cumulative_vested_amount: 0,
            },
            VestingPeriod {
                date: Utc.ymd(2020, 4, 6),
                cumulative_vested_amount: 0,
            },
            VestingPeriod {
                date: Utc.ymd(2020, 5, 6),
                cumulative_vested_amount: 0,
            },
            VestingPeriod {
                date: Utc.ymd(2020, 6, 6),
                cumulative_vested_amount: 0,
            },
            VestingPeriod {
                date: Utc.ymd(2020, 7, 6),
                cumulative_vested_amount: 0,
            },
            VestingPeriod {
                date: Utc.ymd(2020, 8, 6),
                cumulative_vested_amount: 2500,
            },
            VestingPeriod {
                date: Utc.ymd(2020, 9, 6),
                cumulative_vested_amount: 3750,
            },
            VestingPeriod {
                date: Utc.ymd(2020, 10, 6),
                cumulative_vested_amount: 5000,
            },
            VestingPeriod {
                date: Utc.ymd(2020, 11, 6),
                cumulative_vested_amount: 6250,
            },
            VestingPeriod {
                date: Utc.ymd(2020, 12, 6),
                cumulative_vested_amount: 7500,
            },
            VestingPeriod {
                date: Utc.ymd(2021, 1, 6),
                cumulative_vested_amount: 8750,
            },
            VestingPeriod {
                date: Utc.ymd(2021, 2, 6),
                cumulative_vested_amount: 10000,
            },
        ];

        assert_eq!(vesting_schedule.periods, periods)
    }
}
