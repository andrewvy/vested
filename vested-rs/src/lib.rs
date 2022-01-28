use chrono::{Date, Datelike, Utc};

enum VestingScheduleKind {
    Monthly,
}

struct VestingSchedule {
    kind: VestingScheduleKind,
    cliff_percentage: f32,
    cliff: u32,
    length: u32,
}

struct Grant {
    amount: u32,
    grant_date: Date<Utc>,
    vesting_schedule: VestingSchedule,
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
        match self.vesting_schedule.kind {
            VestingScheduleKind::Monthly => {
                if self.is_before_cliff(future_date) {
                    return 0.0;
                } else if self.months_difference(future_date) > self.vesting_schedule.length as i32
                {
                    return self.amount as f32;
                } else {
                    let months_past_cliff =
                        self.months_difference(future_date) - self.vesting_schedule.cliff as i32;

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
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use chrono::TimeZone;

    use super::{Grant, Utc, VestingSchedule, VestingScheduleKind};

    #[test]
    fn it_works() {
        let grant = Grant {
            amount: 10_000,
            grant_date: Utc.ymd(2020, 2, 6),
            vesting_schedule: VestingSchedule {
                kind: VestingScheduleKind::Monthly,
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
}
