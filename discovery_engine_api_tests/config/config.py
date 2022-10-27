from configparser import ConfigParser
import os


class Config:

    def __init__(self):
        configparser = ConfigParser()
        configparser.read(os.path.abspath(os.curdir) + '/config.ini')
        self.ingestion = str(configparser.get('Endpoints', 'ingestion.api'))
        self.interactions = str(configparser.get('Endpoints', 'interactions.api'))
        self.user_id = str(configparser.get('RequestParams', 'user_id'))
