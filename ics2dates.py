# Convert a ics file to the effort format
# Can be used to import a ics with the national holidays for example
import ics
from sys import argv

if len(argv) != 3:
    print(f"USAGE: {argv[0]} ics_file output_file")
    exit(0)

ics_file = argv[1]
output_file = argv[2]

with open(ics_file, 'r') as file:
    calendar = ics.Calendar(file.read())

event_dates = [event.begin.date().strftime('%Y-%m-%d') for event in calendar.events]
event_dates.sort()

with open(output_file, 'w') as file:
    file.write('\n'.join(event_dates))
