from copr.v3 import Client
from pprint import pprint
client = Client.create_from_config_file()
pprint(client.config)


from copr.v3 import config_from_file
config = config_from_file()
client = Client(config)