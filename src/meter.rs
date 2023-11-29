use chrono::{Duration, DateTime, Utc, Datelike};
use std::time::SystemTime;
use rand::Rng;

const WINDOW: i8 = 20;
const KW_RATE: f32 = 20.0;
const KW_RATE_R: f32 = 20.0;

pub struct Meter {
    pub(crate) history: Vec<(DateTime<Utc>, u8,)>,
}
impl Meter {
    pub fn new() -> Self {
        Meter{history: [].to_vec()}
    }
}
pub struct Expenses {
    window_total: f32,
    expenses: Vec<f32>,
    expenses_c1: Vec<f32>,
    expenses_c2: Vec<f32>,
    ratchet_val: Option<(DateTime<Utc>, f32,)>,
    old_ratchet: Option<(DateTime<Utc>, f32,)>,
}
impl Expenses {
    pub fn new() -> Self {
        Expenses {
            window_total: 0.0,
            expenses: [].to_vec(),
            expenses_c1: [].to_vec(),
            expenses_c2: [].to_vec(),
            ratchet_val: None,
            old_ratchet: None,
        }
    }
}
pub struct Client {
    pub(crate) meter: Meter,
    pub(crate) expenses: Expenses,
    pub(crate) _data_array: [(f64, f64);30],
}

impl Client {
    pub fn new() -> Self {
        return Client {
            meter: Meter::new(),
            expenses: Expenses::new(),
            _data_array: (1..=30)
                .into_iter()
                .map(|i| {(0 as f64, i as f64)})
                .collect::<Vec<_>>().try_into().unwrap(),
        }
    }
    pub fn c_update_totals(&mut self) {
        self.expenses.window_total = self.meter.history.iter()
            .rev().take(WINDOW as usize).map(|x| x.1 as f32).sum();
        self.expenses.old_ratchet = self.expenses.ratchet_val;
        let mut latest_history = self.meter.history
            .iter()
            .rev()
            .take(WINDOW as usize * 3)
            .collect::<Vec<_>>();

        latest_history.sort_by_key(|x|x.1);
        self.expenses.ratchet_val = Some((
            latest_history[latest_history.len()-1].0,
            latest_history[latest_history.len()-1].1 as f32,
        ));
        if self.meter.history[self.meter.history.len() - 1].0.day() != 1 {
            return;
        }
        let cost1 = self.expenses.window_total * KW_RATE;
        let cost2 = self.expenses.ratchet_val.unwrap().1 * KW_RATE_R;
        self.expenses.expenses_c1.push(cost1);
        self.expenses.expenses_c2.push(cost2);
    }
    // pub fn get_data(&mut self) -> &[(f64, f64);30] {
    pub fn get_data(&mut self) {
        let v: [(f64, f64);30] =  self.meter.history
            .clone()
            .into_iter().rev()
            .take(30).map(|t| {t.1})
            .enumerate()
            .map(|(i, y)|{((30 - i) as f64, y as f64)})
            .rev()
            .collect::<Vec<_>>().try_into().unwrap();
        self._data_array = v;
    }
}

pub struct MasterMeter {
    pub(crate) meter: Meter,
    expenses: Expenses,
    pub(crate) clients: Vec<Client>,
    expense_factors: Vec<f32>,
}
impl MasterMeter {
    pub fn new(clients: Vec<Client>) -> Self {
        return MasterMeter {
            meter: Meter::new(),
            expenses: Expenses::new(),
            clients: clients,
            expense_factors: [].to_vec(),
        }
    }
    pub fn m_update_totals(&mut self) {
        self.expenses.update_totals(&self.meter);
        if !self.expenses.old_ratchet.is_none() && self.expenses.old_ratchet.unwrap().0 == self.expenses.ratchet_val.unwrap().0 {
            return;
        }
        let mut day_idx: i32 = -1;


        for (i, h) in self.clients[0].meter.history.iter().enumerate() {
            if self.clients[0].meter.history[i].0 == self.expenses.ratchet_val.unwrap().0 {
                day_idx = i as i32;
                break;
            }
        }

        if day_idx == -1 {
            return;
        }
        self.expense_factors = self.clients.iter()
            .map(|client| client.meter.history[day_idx as usize])
            .map(|t| t.1 as f32 / self.expenses.ratchet_val.unwrap().1).collect();
    }
}

pub struct Account {
    balance: f32,
    pub(crate) master_meter: MasterMeter,
    bill_expenses: [f32; 2],
}

impl Account {
    pub fn new(clients: Vec<Client>) -> Self {
        return Account {
            balance: 0.0,
            master_meter: MasterMeter::new(clients),
            bill_expenses: [0.0, 0.0],
        }
    }
}
pub trait Charger {
    fn charge_accounts(&mut self);
}
impl Charger for Account {
    fn charge_accounts(&mut self) {
        self.bill_expenses[0] = self.master_meter.clients.iter()
            .map(|c| c.expenses.expenses_c1.clone())
            .map(|expenses_c1|expenses_c1[expenses_c1.len()-1])
            .sum();
        let ratchet_val_1 = self.master_meter.expenses.ratchet_val.unwrap().1;
        self.bill_expenses[1] = self.master_meter.clients
            .iter()
            .enumerate()
            .map(|(idx, client)|
                self.master_meter.expense_factors[idx] * KW_RATE_R * ratchet_val_1
            ).sum();
        self.balance +=
            self.bill_expenses[0]
            + self.bill_expenses[1]
            - self.master_meter.expenses.expenses[self.master_meter.expenses.expenses.len() - 1];
    }
}

pub trait Reader {
    fn add_point(&mut self);
}

pub trait Update {
    fn update_totals(&mut self, meter: &Meter);
}

impl Update for Expenses {
    fn update_totals(&mut self, meter: &Meter) {
        self.window_total = meter.history.iter()
            .rev().take(WINDOW as usize).map(|x| x.1 as f32).sum();
        self.old_ratchet = self.ratchet_val;
        let mut latest_history = meter.history
            .iter()
            .rev()
            .take(WINDOW as usize * 3)
            .collect::<Vec<_>>();

        latest_history.sort_by_key(|x|x.1);
        self.ratchet_val = Some((
            latest_history[latest_history.len()-1].0,
            latest_history[latest_history.len()-1].1 as f32,
        ));
        if meter.history[meter.history.len() - 1].0.day() != 1 {
            return;
        }
        let cost1 = self.window_total * KW_RATE;
        let cost2 = self.ratchet_val.unwrap().1 * KW_RATE_R;
        self.expenses_c1.push(cost1);
        self.expenses_c2.push(cost2);

    }
}
impl Update for MasterMeter {
    /// Update expense factors and ratchet values.
    fn update_totals(&mut self, meter: &Meter) {
        self.expenses.update_totals(&self.meter);
        if self.expenses.old_ratchet.unwrap().0 == self.expenses.ratchet_val.unwrap().0 {
            return;
        }
        let mut day_idx = 0;
        for (i, h) in self.clients[0].meter.history.iter().enumerate() {
            if self.clients[0].meter.history[i].0 == self.expenses.ratchet_val.unwrap().0 {
                day_idx = i;
                break;
            }
        }
        self.expense_factors = self.clients.iter()
            .map(|client| client.meter.history[day_idx])
            .map(|t| t.1 as f32 / self.expenses.ratchet_val.unwrap().1).collect();
    }
}


impl Reader for Meter {
    /// Add a random point to a meter.
    fn add_point(&mut self) {
        let mut rng = rand::thread_rng();
        let mut t: DateTime<Utc>;
        if self.history.len() > 0 {
            t = self.history[self.history.len()-1].0
        } else {
            t = SystemTime::now().clone().into();
        }

        t = t + Duration::days(1);

        if rng.gen_range(0..20) == 4 {
            self.history.push((t, rng.gen_range(10..20)));
            return;
        }

        if self.history.len() == 0 {
            self.history.push((t, rng.gen_range(2..3)));
            return;
        }
        self.history.push((t, rng.gen_range(5..=30)));
        return;

    }
}

impl Reader for MasterMeter {
    /// Add data from clients into the master meter.
    fn add_point(&mut self) {
        let current_date = self.clients[0]
            .meter.history[self.clients[0].meter.history.len()-1].0;
        let mut total: u8 = 0;
        for b_client in self.clients.iter() {
            let meter = &b_client.meter;
            let hist = &meter.history;
            let hist_len = hist.len();
            let amount = b_client.meter.history[b_client.meter.history.len() - 1].1 as u8;
            total += amount;
        }
        // let total = self.clients.iter()
        //     .map(|client|client.meter.history[client.meter.history.len()-1].1).sum();
        self.meter.history.push((current_date, total));
    }
}