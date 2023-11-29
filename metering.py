import dataclasses
import random
import os
import sys
import time
import typing
from datetime import datetime, timedelta
from typing import Iterable, List, Tuple, Any


def generate_next_point(meter: 'ClientMeter'):
    t = meter.history[-1][0] if meter.history else datetime.now()
    next_day = t + timedelta(days=1)
    if random.randint(0, 20) == 4:
        return next_day, random.randint(10, 20)
    if not meter.history:
        return next_day, random.randint(2, 3)

    current_value = meter.history[-1][1]
    if current_value == 9:
        return next_day, current_value - random.randint(0, 1)
    elif current_value == 0:
        return next_day, current_value + random.randint(0, 1)
    elif current_value < 10:
        return next_day, current_value + random.randint(-1, 1)
    else:
        return next_day, random.randint(2, 8)


class BaseMeter:
    """A simple meter."""
    history: List[Tuple[Any, int]] = []
    KW_RATE = 0.8
    KW_RATE_R = 1.2

    def __init__(self, name, slaves=None):
        """Initialize."""
        self.name = name
        self.history = []

    def add_point(self, t: datetime, v: int):
        self.history.append((t, v))


class ClientMeter(BaseMeter):
    """A client meter."""

    def __init__(self, name, slaves=None):
        """Initialize."""
        super().__init__(name)
        self.window_total = 0
        self.expenses: typing.List[float] = [0]
        self.expenses_c1: typing.List[float] = [0]
        self.expenses_c2: typing.List[float] = [0]
        self.ratchet_val: typing.Optional[typing.Tuple[datetime, float]] = None
        self.old_ratchet = None

    def update_totals(self, window):
        self.window_total = sum([c[1] for c in self.history[-1 * window:]])
        self.old_ratchet = self.ratchet_val
        self.ratchet_val = max([c for c in self.history[-1 * 3 * window:]], key=lambda x: x[1])
        if self.history[-1][0].day != 1:
            return
        cost1 = self.window_total * self.KW_RATE
        cost2 = self.ratchet_val[1] * self.KW_RATE_R
        self.expenses_c1.append(cost1)
        self.expenses_c2.append(cost2)
        self.expenses.append(cost1 + cost2)

    def draw(self, window):
        history_length = len(self.history)
        lanes = [{k: ' ' for k in range(0, 20)} for _ in range(0, history_length)]
        for idx, history_point in enumerate(self.history):
            day_value = history_point[1]
            height = int(day_value / 3)
            if day_value % 3 == 0:
                lanes[idx][height] = '_'
            elif day_value % 3 == 1:
                lanes[idx][height] = '-'
            elif day_value % 3 == 2:
                lanes[idx][height] = '^'
        strings = []
        for k in range(0, 8):
            strings.append(''.join([lanes[l_idx][k] for l_idx in range(history_length-window, history_length)]))
        cost1 = self.window_total*self.KW_RATE
        cost2 = self.ratchet_val[1]*self.KW_RATE_R if self.ratchet_val else 0
        return ('\n'.join(reversed(strings)) + '\n' 
        + ''.join(['=']*(window-10))
        + f'Costs: [kwd] ${cost1:.1f}\t[kw-Max] ${cost2:.1f}  [Live]: {self.history[-1][1]:2} [Total]: {self.expenses[-1]:.1f}' + '\n')


class MasterMeter(ClientMeter):
    """
    Keeps an expense factor for each client meter.

    The expense factor is used for charging clients a proportional amount when there is a spike in demand.
    """

    def __init__(self, *args, **kwargs):
        self.clients = kwargs.pop('clients')
        self.expense_factors = []
        super().__init__(*args, **kwargs)

    def update_totals(self, window):
        super().update_totals(window)
        self.update_expense_factors()

    def update_expense_factors(self):
        """
        Update expense factors if there is a new ratchet max.
        :return:
        """
        if self.old_ratchet is not None and self.old_ratchet[0] == self.ratchet_val[0]:
            return

        day_idx = 0

        # Get the ratchet_val date as an offset.
        for i, h in enumerate(self.clients[0].history):
            if self.clients[0].history[i][0] == self.ratchet_val[0]:
                day_idx = i
                break
        # Get the metered value for all meters at that date.
        self.expense_factors = [
            round(float(m.history[day_idx][1]) / float(self.ratchet_val[1]), 2)
            for m in self.clients
        ]


@dataclasses.dataclass
class AccountManager:
    """
    Container for the company balance, master meter, and client meters.
    """
    company_balance: float
    master: MasterMeter
    meters: typing.List[ClientMeter]
    bill_expenses_1 = 0
    bill_expenses_2 = 0

    def __post_init__(self):
        self.bill_expenses_1 = 0
        self.bill_expenses_2 = 0

    def charge_accounts(self):
        """
        Sum up the latest expenses from client meters and add them to company_balance.
        """
        self.bill_expenses_1 = sum([m.expenses_c1[-1] for m in self.meters])
        self.bill_expenses_2 = sum(
            [self.master.ratchet_val[1] * self.master.expense_factors[i] * self.master.KW_RATE_R
             for i, _ in enumerate(self.meters)]
        )
        self.company_balance += self.bill_expenses_1 + self.bill_expenses_2 - self.master.expenses[-1]


if __name__ == '__main__':
    """
    Run program.
    
    Initializes meters and gives each 30 days of data.
    
    Initializes a master_meter which is a sum aggregation of all other meters..
    """
    meters = [ClientMeter(f'meter{x}') for x in range(0, 3)]
    master_meter = MasterMeter(f'masterMeter', clients=meters)
    for i, m in enumerate(meters):
        for _ in range(0, 30):
            m.add_point(*generate_next_point(m))
    
    for i in range(0, len(meters[0].history)):
        the_date = meters[0].history[i][0]
        the_sum = sum([the_meter.history[i][1] for the_meter in meters])
        master_meter.add_point(the_date, the_sum)

    window = 20
    DAYS = 365*20

    account_manager = AccountManager(100, master_meter, meters)
    
    for y in range(0, DAYS):
        
        sums1 = 0
        for i, m in enumerate(meters):
            m.add_point(*generate_next_point(m))
            m.update_totals(window)
        current_date = meters[0].history[-1][0]
        current_total_kw = sum([the_meter.history[-1][1] for the_meter in meters])
        master_meter.add_point(current_date, current_total_kw)
        master_meter.update_totals(window)

        
        
        # If 2nd of month, assume electricity was paid and clients paid bills
        if master_meter.history[-1][0].day == 2:
            account_manager.charge_accounts()

        # Draw everything every 3 days.
        if y % 3 == 1:
        # if True:
            os.system('clear')
            # print meters
            for i, m in enumerate(meters):
                sys.stdout.write(m.draw(window))
            sys.stdout.write(master_meter.draw(window))

            # print total
            aggregate = sum([m.expenses[-1] for m in meters])
            sys.stdout.write((''.join(['\t']*(7))+'   '
            + f'[Total]: {aggregate:.1f} (agg cost to tenants)' + '\n'))

            # print weighted aggregate total
            sys.stdout.write((''.join(['\t']*(7))+'   '
            + f'[Total]: {(account_manager.bill_expenses_1 + account_manager.bill_expenses_2):.1f} (new cost to tenants)' + '\n'))
            current_date = master_meter.history[-1][0].strftime('%Y/%m/%d')
            sys.stdout.write(f'\n\t\t\t\t\t[Balance ({current_date})]:{account_manager.company_balance:.1f}')
            
            sys.stdout.flush()            
        
        time.sleep(0.04)


